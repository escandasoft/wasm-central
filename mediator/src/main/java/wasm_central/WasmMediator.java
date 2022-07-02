package wasm_central;

import io.quarkus.runtime.QuarkusApplication;
import io.quarkus.runtime.annotations.QuarkusMain;
import picocli.CommandLine;

import java.util.concurrent.Callable;

@QuarkusMain
public class WasmMediator implements QuarkusApplication {
    @CommandLine.Command(name = "wasm-mediator", mixinStandardHelpOptions = true, version = "wasm-mediator 0.1.0",
            description = "Listens for Kafka and executes a WASM module on topic messages.")
    static class Program implements Callable<Integer> {
        @CommandLine.Parameters(description = "The host address to listen to.")
        private String host;

        @CommandLine.Parameters(description = "The port to listen to.")
        private int port;

        @CommandLine.Option(names = {"-fh", "--fn-host"}, description = "Functions host")
        private String functionsHost;

        @CommandLine.Option(names = {"-fp", "--fn-port"}, description = "Functions port")
        private int functionsPort;

        @Override
        public Integer call() throws Exception {
            System.out.println("Hello world");
            return 0;
        }
    }

    @Override
    public int run(String... args) throws Exception {
        new CommandLine(new Program()).execute(args);
        return 0;
    }
}
