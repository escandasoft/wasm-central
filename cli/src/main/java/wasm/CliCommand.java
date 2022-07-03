package wasm;

import io.quarkus.picocli.runtime.annotations.TopCommand;
import picocli.CommandLine;

@TopCommand
@CommandLine.Command(name = "wasm-central-cli", mixinStandardHelpOptions = true, subcommands = {FunctionCommand.class})
public class CliCommand {
}
