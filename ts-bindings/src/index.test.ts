import { describe, expect, test } from "bun:test"
import { mkdtempSync, readFileSync, rmSync } from "node:fs"
import { join, resolve } from "node:path"
import { tmpdir } from "node:os"

import {
	HostClosedStreamError,
	PLUGIN_PROTOCOL_VERSION,
	parseHostRpcMessage,
	parsePluginRpcMessage,
	PluginIdMismatchError,
	PluginSession,
	ProtocolVersionMismatchError,
	type LineReader,
	type LineWriter,
	type PluginMetadata,
	type PluginRpcMessage,
	serializePluginRpcMessage,
	UnexpectedMessageError,
} from "./index"

class MemoryTransport implements LineReader, LineWriter {
	readonly writes: string[] = []

	constructor(private readonly reads: Array<string | null>) {}

	async readLine(): Promise<string | null> {
		return this.reads.shift() ?? null
	}

	async writeLine(line: string): Promise<void> {
		this.writes.push(line)
	}
}

const metadata: PluginMetadata = {
	pluginId: "example.hello",
	name: "Hello Plugin",
	version: "0.1.0",
	capabilities: ["command_provider"],
}

describe("PluginSession", () => {
	test("parses host hello", () => {
		expect(
			parseHostRpcMessage(
				'{"type":"hello","payload":{"protocol_version":1,"host_name":"termy","host_version":"0.1.44","plugin_id":"example.hello"}}',
			),
		).toEqual({
			type: "hello",
			payload: {
				protocol_version: 1,
				host_name: "termy",
				host_version: "0.1.44",
				plugin_id: "example.hello",
			},
		})
	})

	test("serializes plugin hello during initialization", async () => {
		const transport = new MemoryTransport([
			'{"type":"hello","payload":{"protocol_version":1,"host_name":"termy","host_version":"0.1.44","plugin_id":"example.hello"}}',
		])

		const session = await PluginSession.initialize(transport, transport, metadata)

		expect(session.pluginId).toBe("example.hello")
		expect(transport.writes).toEqual([
			'{"type":"hello","payload":{"protocol_version":1,"plugin_id":"example.hello","name":"Hello Plugin","version":"0.1.0","capabilities":["command_provider"]}}',
		])
	})

	test("rejects protocol version mismatch", async () => {
		const transport = new MemoryTransport([
			'{"type":"hello","payload":{"protocol_version":9,"host_name":"termy","host_version":"0.1.44","plugin_id":"example.hello"}}',
		])

		await expect(
			PluginSession.initialize(transport, transport, metadata),
		).rejects.toBeInstanceOf(ProtocolVersionMismatchError)
	})

	test("rejects plugin id mismatch", async () => {
		const transport = new MemoryTransport([
			'{"type":"hello","payload":{"protocol_version":1,"host_name":"termy","host_version":"0.1.44","plugin_id":"wrong.id"}}',
		])

		await expect(
			PluginSession.initialize(transport, transport, metadata),
		).rejects.toBeInstanceOf(PluginIdMismatchError)
	})

	test("extracts command id from invoke_command", () => {
		const commandId = PluginSession.commandId({
			type: "invoke_command",
			payload: { command_id: "view.releaseNotes" },
		})

		expect(commandId).toBe("view.releaseNotes")
		expect(PluginSession.commandId({ type: "ping" })).toBeUndefined()
	})

	test("sendLog, sendToast, and sendPong preserve the wire shape", async () => {
		const transport = new MemoryTransport([
			'{"type":"hello","payload":{"protocol_version":1,"host_name":"termy","host_version":"0.1.44","plugin_id":"example.hello"}}',
		])
		const session = await PluginSession.initialize(transport, transport, metadata)

		transport.writes.length = 0
		await session.sendLog("info", "hello")
		await session.sendToast("success", "toast body", 1200)
		await session.sendPong()

		expect(transport.writes).toEqual([
			'{"type":"log","payload":{"level":"info","message":"hello"}}',
			'{"type":"toast","payload":{"level":"success","message":"toast body","duration_ms":1200}}',
			'{"type":"pong"}',
		])
	})

	test("runUntilShutdown stops after shutdown", async () => {
		const transport = new MemoryTransport([
			'{"type":"hello","payload":{"protocol_version":1,"host_name":"termy","host_version":"0.1.44","plugin_id":"example.hello"}}',
			'{"type":"ping"}',
			'{"type":"invoke_command","payload":{"command_id":"view.releaseNotes"}}',
			'{"type":"shutdown"}',
			'{"type":"ping"}',
		])
		const session = await PluginSession.initialize(transport, transport, metadata)
		const seen: string[] = []

		await session.runUntilShutdown(async (message) => {
			seen.push(message.type)
		})

		expect(seen).toEqual(["ping", "invoke_command", "shutdown"])
	})

	test("rejects unexpected initial message", async () => {
		const transport = new MemoryTransport(['{"type":"ping"}'])

		await expect(
			PluginSession.initialize(transport, transport, metadata),
		).rejects.toBeInstanceOf(UnexpectedMessageError)
	})

	test("throws when host closes the stream", async () => {
		const transport = new MemoryTransport([null])

		await expect(
			PluginSession.initialize(transport, transport, metadata),
		).rejects.toBeInstanceOf(HostClosedStreamError)
	})
})

describe("protocol compatibility fixtures", () => {
	test("matches Rust host hello JSON fixture", () => {
		const fixture =
			'{"type":"hello","payload":{"protocol_version":1,"host_name":"termy","host_version":"0.1.44","plugin_id":"example.hello"}}'

		expect(parseHostRpcMessage(fixture)).toEqual({
			type: "hello",
			payload: {
				protocol_version: 1,
				host_name: "termy",
				host_version: "0.1.44",
				plugin_id: "example.hello",
			},
		})
	})

	test("matches Rust plugin hello JSON fixture", () => {
		const fixture =
			'{"type":"hello","payload":{"protocol_version":1,"plugin_id":"example.hello","name":"Hello Plugin","version":"0.1.0","capabilities":["command_provider"]}}'

		expect(parsePluginRpcMessage(fixture)).toEqual({
			type: "hello",
			payload: {
				protocol_version: 1,
				plugin_id: "example.hello",
				name: "Hello Plugin",
				version: "0.1.0",
				capabilities: ["command_provider"],
			},
		})
	})

	test("serializes exact plugin hello fixture", () => {
		const message: PluginRpcMessage = {
			type: "hello",
			payload: {
				protocol_version: PLUGIN_PROTOCOL_VERSION,
				plugin_id: "example.hello",
				name: "Hello Plugin",
				version: "0.1.0",
				capabilities: ["command_provider"],
			},
		}

		expect(serializePluginRpcMessage(message)).toBe(
			'{"type":"hello","payload":{"protocol_version":1,"plugin_id":"example.hello","name":"Hello Plugin","version":"0.1.0","capabilities":["command_provider"]}}',
		)
	})
})

describe("examples/hello-plugin", () => {
	test("ships a complete manifest aligned with host expectations", () => {
		const manifestPath = resolve(
			import.meta.dir,
			"../examples/hello-plugin/termy-plugin.json",
		)
		const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as Record<
			string,
			unknown
		>

		expect(manifest).toEqual({
			schema_version: 1,
			id: "example.hello",
			name: "Hello Plugin",
			version: "0.1.0",
			description: "Minimal TypeScript Termy plugin compiled with Bun",
			runtime: "executable",
			entrypoint: "./dist/hello-plugin",
			autostart: true,
			permissions: ["notifications"],
			contributes: {
				commands: [
					{
						id: "example.hello.run",
						title: "Run Hello",
						description:
							"Show a sample toast from the TypeScript example plugin",
					},
				],
			},
		})
	})

	test("compiles and responds like a real executable plugin", async () => {
		const outDir = mkdtempSync(join(tmpdir(), "termy-ts-plugin-"))
		const outfile = join(outDir, "hello-plugin")
		const source = resolve(import.meta.dir, "../examples/hello-plugin/plugin.ts")

		try {
			const build = Bun.spawn(
				["bun", "build", "--compile", source, "--outfile", outfile],
				{
					stdout: "pipe",
					stderr: "pipe",
				},
			)
			expect(await build.exited).toBe(0)

			const child = Bun.spawn([outfile], {
				stdin: "pipe",
				stdout: "pipe",
				stderr: "pipe",
			})

			const decoder = new TextDecoder()

			await Promise.resolve(
				child.stdin.write(
				new TextEncoder().encode(
					'{"type":"hello","payload":{"protocol_version":1,"host_name":"termy","host_version":"0.1.44","plugin_id":"example.hello"}}\n',
				),
			)
			)

			const helloLine = await readNextLine(child.stdout)
			expect(helloLine).toBe(
				'{"type":"hello","payload":{"protocol_version":1,"plugin_id":"example.hello","name":"Hello Plugin","version":"0.1.0","capabilities":["command_provider"]}}',
			)

			await Promise.resolve(
				child.stdin.write(new TextEncoder().encode('{"type":"ping"}\n')),
			)
			expect(await readNextLine(child.stdout)).toBe('{"type":"pong"}')

			await Promise.resolve(
				child.stdin.write(
				new TextEncoder().encode(
					'{"type":"invoke_command","payload":{"command_id":"example.hello.run"}}\n',
				),
			)
			)
			expect(await readNextLine(child.stdout)).toBe(
				'{"type":"log","payload":{"level":"info","message":"Running hello command"}}',
			)
			expect(await readNextLine(child.stdout)).toBe(
				'{"type":"toast","payload":{"level":"success","message":"Hello from TypeScript","duration_ms":2000}}',
			)

			await Promise.resolve(
				child.stdin.write(new TextEncoder().encode('{"type":"shutdown"}\n')),
			)
			await Promise.resolve(child.stdin.end())
			expect(await child.exited).toBe(0)
			expect(decoder.decode(await new Response(child.stderr).arrayBuffer())).toBe("")
		} finally {
			rmSync(outDir, { recursive: true, force: true })
		}
	})
})

async function readNextLine(stream: ReadableStream<Uint8Array>): Promise<string> {
	const reader = stream.getReader()
	const chunks: Uint8Array[] = []

	try {
		for (;;) {
			const { value, done } = await reader.read()
			if (done) {
				break
			}
			chunks.push(value)
			const decoded = new TextDecoder().decode(concatChunks(chunks))
			const newlineIndex = decoded.indexOf("\n")
			if (newlineIndex >= 0) {
				return decoded.slice(0, newlineIndex)
			}
		}
	} finally {
		reader.releaseLock()
	}

	throw new Error("stream closed before newline")
}

function concatChunks(chunks: Uint8Array[]): Uint8Array {
	const length = chunks.reduce((total, chunk) => total + chunk.length, 0)
	const merged = new Uint8Array(length)
	let offset = 0
	for (const chunk of chunks) {
		merged.set(chunk, offset)
		offset += chunk.length
	}
	return merged
}
