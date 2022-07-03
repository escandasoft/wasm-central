package wasm;

import picocli.CommandLine;

@CommandLine.Command(name = "function")
public class FunctionCommand {
    @CommandLine.Command(name = "list")
    public int list(@CommandLine.Option(names = "-H") String host,
                      @CommandLine.Option(names = "-P") int port) {
        System.out.println("!! listing functions");
        return 0;
    }

    @CommandLine.Command(name = "deploy")
    public int deploy(@CommandLine.Option(names = "-H") String host,
                      @CommandLine.Option(names = "-P") int port,
                      @CommandLine.Option(names = "-F") String file,
                      @CommandLine.Option(names = "-I") String inputs,
                      @CommandLine.Option(names = "-O") String outputs) {
        System.out.println("!! deploying function");
        return 0;
    }
}
