package wasm_central;

import io.quarkus.runtime.Quarkus;
import io.quarkus.runtime.QuarkusApplication;
import io.quarkus.runtime.annotations.QuarkusMain;
import picocli.CommandLine;

import java.util.concurrent.Callable;

@QuarkusMain
@CommandLine.Command(name = "wasm-mediator", mixinStandardHelpOptions = true, version = "wasm-mediator 0.1.0",
        description = "Listens for Kafka and executes a WASM module on topic messages.")
public class WasmMediator implements QuarkusApplication, Callable<Integer> {
    @CommandLine.Option(names = {"-fh", "--fn-host"}, description = "Functions host")
    private String functionsHost;

    @CommandLine.Option(names = {"-fp", "--fn-port"}, description = "Functions port")
    private int functionsPort;

    @Override
    public Integer call() throws Exception {
        return 0;
    }

    @Override
    public int run(String... args) throws Exception {
        int ret = new CommandLine(this).execute(args);
        Quarkus.waitForExit();
        return ret;
    }
}
