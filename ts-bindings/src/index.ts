import { createInterface, type Interface as ReadLineInterface } from "readline";

export const PLUGIN_MANIFEST_FILE_NAME = "termy-plugin.json";
export const PLUGIN_PROTOCOL_VERSION = 1;

export type PluginRuntime = "executable";

export type PluginPermission =
  | "filesystem_read"
  | "filesystem_write"
  | "network"
  | "shell"
  | "clipboard"
  | "notifications"
  | "terminal_read"
  | "terminal_write"
  | "ui_panels";

export interface PluginContributions {
  commands?: PluginCommandContribution[];
}

export interface PluginCommandContribution {
  id: string;
  title: string;
  description?: string;
}

export interface PluginManifest {
  schema_version: number;
  id: string;
  name: string;
  version: string;
  description?: string;
  author?: string;
  minimum_host_version?: string;
  api_version?: number;
  runtime?: PluginRuntime;
  entrypoint: string;
  autostart?: boolean;
  permissions?: PluginPermission[];
  contributes?: PluginContributions;
}

export interface HostHello {
  protocol_version: number;
  host_name: string;
  host_version: string;
  plugin_id: string;
}

export interface HostCommandInvocation {
  command_id: string;
}

export type HostRpcMessage =
  | { type: "hello"; payload: HostHello }
  | { type: "invoke_command"; payload: HostCommandInvocation }
  | { type: "shutdown" }
  | { type: "ping" };

export type PluginCapability =
  | "command_provider"
  | "event_subscriber"
  | "ui_panel";

export interface PluginHello {
  protocol_version: number;
  plugin_id: string;
  name: string;
  version: string;
  capabilities?: PluginCapability[];
}

export type PluginLogLevel = "trace" | "debug" | "info" | "warn" | "error";

export interface PluginLogMessage {
  level: PluginLogLevel;
  message: string;
}

export type PluginToastLevel = "info" | "success" | "warning" | "error";

export interface PluginToastMessage {
  level: PluginToastLevel;
  message: string;
  duration_ms?: number;
}

export type PluginRpcMessage =
  | { type: "hello"; payload: PluginHello }
  | { type: "log"; payload: PluginLogMessage }
  | { type: "toast"; payload: PluginToastMessage }
  | { type: "pong" };

export interface PluginMetadata {
  pluginId: string;
  name: string;
  version: string;
  capabilities?: PluginCapability[];
}

export interface LineReader {
  readLine(): Promise<string | null>;
}

export interface LineWriter {
  writeLine(line: string): Promise<void>;
}

export class PluginSessionError extends Error {
  constructor(message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = "PluginSessionError";
  }
}

export class HostClosedStreamError extends PluginSessionError {
  constructor() {
    super("host closed the plugin stream");
    this.name = "HostClosedStreamError";
  }
}

export class ProtocolVersionMismatchError extends PluginSessionError {
  readonly expected: number;
  readonly actual: number;

  constructor(expected: number, actual: number) {
    super(`protocol version mismatch: expected ${expected}, got ${actual}`);
    this.name = "ProtocolVersionMismatchError";
    this.expected = expected;
    this.actual = actual;
  }
}

export class PluginIdMismatchError extends PluginSessionError {
  readonly expected: string;
  readonly actual: string;

  constructor(expected: string, actual: string) {
    super(`plugin id mismatch: expected \`${expected}\`, got \`${actual}\``);
    this.name = "PluginIdMismatchError";
    this.expected = expected;
    this.actual = actual;
  }
}

export class InvalidMessageError extends PluginSessionError {
  readonly line: string;

  constructor(line: string, cause?: unknown) {
    super(`invalid message: ${line}`, cause ? { cause } : undefined);
    this.name = "InvalidMessageError";
    this.line = line;
  }
}

export class UnexpectedMessageError extends PluginSessionError {
  constructor(message: string) {
    super(message);
    this.name = "UnexpectedMessageError";
  }
}

class ReadlineTransport implements LineReader, LineWriter {
  private readonly lineReader: AsyncIterator<string>;
  private readonly readline: ReadLineInterface;

  constructor(
    private readonly input: NodeJS.ReadableStream,
    private readonly output: NodeJS.WritableStream,
  ) {
    this.readline = createInterface({
      input: this.input,
      crlfDelay: Infinity,
      terminal: false,
    });
    this.lineReader = this.readline[Symbol.asyncIterator]();
  }

  async readLine(): Promise<string | null> {
    const { value, done } = await this.lineReader.next();
    return done ? null : value;
  }

  async writeLine(line: string): Promise<void> {
    await new Promise<void>((resolve, reject) => {
      this.output.write(`${line}\n`, (error?: Error | null) => {
        if (error) {
          reject(error);
          return;
        }
        resolve();
      });
    });
  }

  close(): void {
    this.readline.close();
  }
}

export class PluginSession {
  private constructor(
    private readonly reader: LineReader,
    private readonly writer: LineWriter,
    readonly hostHello: HostHello,
    readonly pluginId: string,
  ) {}

  static async stdio(metadata: PluginMetadata): Promise<PluginSession> {
    const transport = new ReadlineTransport(process.stdin, process.stdout);
    try {
      return await PluginSession.initialize(transport, transport, metadata);
    } catch (error) {
      transport.close();
      throw error;
    }
  }

  static async initialize(
    reader: LineReader,
    writer: LineWriter,
    metadata: PluginMetadata,
  ): Promise<PluginSession> {
    const helloMessage = await readHostHello(reader);

    if (helloMessage.protocol_version !== PLUGIN_PROTOCOL_VERSION) {
      throw new ProtocolVersionMismatchError(
        PLUGIN_PROTOCOL_VERSION,
        helloMessage.protocol_version,
      );
    }

    if (helloMessage.plugin_id !== metadata.pluginId) {
      throw new PluginIdMismatchError(
        metadata.pluginId,
        helloMessage.plugin_id,
      );
    }

    const pluginId = helloMessage.plugin_id;
    await writePluginMessage(writer, {
      type: "hello",
      payload: {
        protocol_version: PLUGIN_PROTOCOL_VERSION,
        plugin_id: pluginId,
        name: metadata.name,
        version: metadata.version,
        capabilities: metadata.capabilities ?? [],
      },
    });

    return new PluginSession(reader, writer, helloMessage, pluginId);
  }

  async recv(): Promise<HostRpcMessage> {
    return readHostMessage(this.reader);
  }

  async send(message: PluginRpcMessage): Promise<void> {
    await writePluginMessage(this.writer, message);
  }

  async sendLog(level: PluginLogLevel, message: string): Promise<void> {
    await this.send({
      type: "log",
      payload: {
        level,
        message,
      },
    });
  }

  async sendPong(): Promise<void> {
    await this.send({ type: "pong" });
  }

  async sendToast(
    level: PluginToastLevel,
    message: string,
    durationMs?: number,
  ): Promise<void> {
    await this.send({
      type: "toast",
      payload: {
        level,
        message,
        duration_ms: durationMs,
      },
    });
  }

  static commandId(message: HostRpcMessage): string | undefined {
    return message.type === "invoke_command"
      ? message.payload.command_id
      : undefined;
  }

  async runUntilShutdown(
    onMessage: (
      message: HostRpcMessage,
      session: PluginSession,
    ) => void | Promise<void>,
  ): Promise<void> {
    for (;;) {
      const message = await this.recv();
      const shouldStop = message.type === "shutdown";
      await onMessage(message, this);
      if (shouldStop) {
        return;
      }
    }
  }
}

export async function readHostHello(reader: LineReader): Promise<HostHello> {
  const message = await readHostMessage(reader);
  if (message.type !== "hello") {
    throw new UnexpectedMessageError(
      `expected host hello, got ${describeHostMessage(message)}`,
    );
  }
  return message.payload;
}

export async function readHostMessage(
  reader: LineReader,
): Promise<HostRpcMessage> {
  const line = await reader.readLine();
  if (line === null) {
    throw new HostClosedStreamError();
  }
  return parseHostRpcMessage(line);
}

export function parseHostRpcMessage(line: string): HostRpcMessage {
  let value: unknown;
  try {
    value = JSON.parse(line);
  } catch (error) {
    throw new InvalidMessageError(line, error);
  }
  return parseHostRpcValue(value, line);
}

export function parsePluginRpcMessage(line: string): PluginRpcMessage {
  let value: unknown;
  try {
    value = JSON.parse(line);
  } catch (error) {
    throw new InvalidMessageError(line, error);
  }
  return parsePluginRpcValue(value, line);
}

export function serializePluginRpcMessage(message: PluginRpcMessage): string {
  validatePluginRpcMessage(message, JSON.stringify(message));
  return JSON.stringify(message);
}

async function writePluginMessage(
  writer: LineWriter,
  message: PluginRpcMessage,
): Promise<void> {
  await writer.writeLine(serializePluginRpcMessage(message));
}

function parseHostRpcValue(value: unknown, line: string): HostRpcMessage {
  const record = asRecord(value, line);
  const type = readString(record, "type", line);

  switch (type) {
    case "hello":
      return {
        type,
        payload: parseHostHello(readPayload(record, line), line),
      };
    case "invoke_command":
      return {
        type,
        payload: parseHostCommandInvocation(readPayload(record, line), line),
      };
    case "shutdown":
      return { type };
    case "ping":
      return { type };
    default:
      throw new InvalidMessageError(line);
  }
}

function parsePluginRpcValue(value: unknown, line: string): PluginRpcMessage {
  const record = asRecord(value, line);
  const type = readString(record, "type", line);

  switch (type) {
    case "hello":
      return {
        type,
        payload: parsePluginHello(readPayload(record, line), line),
      };
    case "log":
      return {
        type,
        payload: parsePluginLogMessage(readPayload(record, line), line),
      };
    case "toast":
      return {
        type,
        payload: parsePluginToastMessage(readPayload(record, line), line),
      };
    case "pong":
      return { type };
    default:
      throw new InvalidMessageError(line);
  }
}

function parseHostHello(value: unknown, line: string): HostHello {
  const record = asRecord(value, line);
  return {
    protocol_version: readNumber(record, "protocol_version", line),
    host_name: readString(record, "host_name", line),
    host_version: readString(record, "host_version", line),
    plugin_id: readString(record, "plugin_id", line),
  };
}

function parseHostCommandInvocation(
  value: unknown,
  line: string,
): HostCommandInvocation {
  const record = asRecord(value, line);
  return {
    command_id: readString(record, "command_id", line),
  };
}

function parsePluginHello(value: unknown, line: string): PluginHello {
  const record = asRecord(value, line);
  return {
    protocol_version: readNumber(record, "protocol_version", line),
    plugin_id: readString(record, "plugin_id", line),
    name: readString(record, "name", line),
    version: readString(record, "version", line),
    capabilities: readOptionalStringArray(record, "capabilities", line) as
      | PluginCapability[]
      | undefined,
  };
}

function parsePluginLogMessage(value: unknown, line: string): PluginLogMessage {
  const record = asRecord(value, line);
  return {
    level: readString(record, "level", line) as PluginLogLevel,
    message: readString(record, "message", line),
  };
}

function parsePluginToastMessage(
  value: unknown,
  line: string,
): PluginToastMessage {
  const record = asRecord(value, line);
  return {
    level: readString(record, "level", line) as PluginToastLevel,
    message: readString(record, "message", line),
    duration_ms: readOptionalNumber(record, "duration_ms", line),
  };
}

function validatePluginRpcMessage(
  message: PluginRpcMessage,
  line: string,
): void {
  parsePluginRpcValue(message, line);
}

function describeHostMessage(message: HostRpcMessage): string {
  return message.type;
}

function asRecord(value: unknown, line: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new InvalidMessageError(line);
  }
  return value as Record<string, unknown>;
}

function readPayload(record: Record<string, unknown>, line: string): unknown {
  if (!("payload" in record)) {
    throw new InvalidMessageError(line);
  }
  return record.payload;
}

function readString(
  record: Record<string, unknown>,
  key: string,
  line: string,
): string {
  const value = record[key];
  if (typeof value !== "string") {
    throw new InvalidMessageError(line);
  }
  return value;
}

function readNumber(
  record: Record<string, unknown>,
  key: string,
  line: string,
): number {
  const value = record[key];
  if (typeof value !== "number" || Number.isNaN(value)) {
    throw new InvalidMessageError(line);
  }
  return value;
}

function readOptionalNumber(
  record: Record<string, unknown>,
  key: string,
  line: string,
): number | undefined {
  const value = record[key];
  if (value === undefined) {
    return undefined;
  }
  if (typeof value !== "number" || Number.isNaN(value)) {
    throw new InvalidMessageError(line);
  }
  return value;
}

function readOptionalStringArray(
  record: Record<string, unknown>,
  key: string,
  line: string,
): string[] | undefined {
  const value = record[key];
  if (value === undefined) {
    return undefined;
  }
  if (
    !Array.isArray(value) ||
    value.some((entry) => typeof entry !== "string")
  ) {
    throw new InvalidMessageError(line);
  }
  return [...value];
}
