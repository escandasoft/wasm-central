package wasm.args;

import picocli.CommandLine;
import wasm.args.fn.*;

@CommandLine.Command(name = "function", subcommands = {ListCommand.class, InvokeCommand.class, DeployCommand.class})
public class FunctionCommand {
}
