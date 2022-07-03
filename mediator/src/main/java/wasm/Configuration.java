package wasm_central;

import io.quarkus.runtime.Startup;
import picocli.CommandLine;
import wasm_central.conf.FnHostRemote;

import javax.enterprise.context.ApplicationScoped;
import javax.enterprise.inject.Produces;
import javax.inject.Singleton;

@Startup
@ApplicationScoped
public class Configuration {
    @Produces
    @Singleton
    FnHostRemote fnRemoteConf(CommandLine.ParseResult cmdParseResult) {
        String fnHost = cmdParseResult.matchedOption("fn-host").getValue().toString();
        String fnPort = cmdParseResult.matchedOption("fn-port").getValue().toString();
        int port = Integer.parseInt(fnPort);
        return new FnHostRemote(fnHost, port);
    }
}
