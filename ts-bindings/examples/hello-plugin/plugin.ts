import { PluginSession, type PluginMetadata } from "../../src/index";

const COMMAND_ID = "example.hello.run";

const metadata: PluginMetadata = {
  pluginId: "example.hello",
  name: "Hello Plugin",
  version: "0.1.0",
  capabilities: ["command_provider"],
};

const session = await PluginSession.stdio(metadata);

await session.runUntilShutdown(async (message, currentSession) => {
  switch (message.type) {
    case "ping":
      await currentSession.sendPong();
      break;
    case "invoke_command":
      if (message.payload.command_id === COMMAND_ID) {
        await currentSession.sendLog("info", "Running hello command");
        await currentSession.sendToast(
          "success",
          "Hello from TypeScript",
          2000,
        );
      }
      break;
    case "shutdown":
      break;
    case "hello":
      break;
  }
});
