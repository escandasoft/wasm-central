package wasm.args.fn;

import com.google.protobuf.ByteString;
import io.quarkus.grpc.GrpcClient;
import io.smallrye.mutiny.Multi;
import mgmt_proto.Manager;
import mgmt_proto.Mgmt;
import picocli.CommandLine;

import javax.enterprise.inject.Instance;
import java.io.FileInputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.Optional;
import java.util.concurrent.Callable;

@CommandLine.Command(name = "deploy")
public class DeployCommand implements Callable<Integer> {
    @CommandLine.Option(names = {"-n", "--name"}, description = "Function name", required = true) String name;

    @CommandLine.Option(names = {"-f", "--file"}, description = "Runnable WASM file", required = true) String file;

    @CommandLine.Option(names = {"-i", "--inputs"}, description = "Input topics", required = true) String inputs;

    @CommandLine.Option(names = {"-o", "--outputs"}, description = "Output topics", required = true) String outputs;

    @GrpcClient
    Instance<Manager> manager;


    @Override
    public Integer call() throws Exception {
        System.out.println("!! deploying function");
        var BUFFER_SIZE = 1024 * 1024;
        var filePath = Path.of(file);
        var pathFile = filePath.toFile();
        try (FileInputStream is = new FileInputStream(pathFile)) {
            System.out.println("!! deploying function named '" + name + "'");
            var multi = Multi.createFrom().range(0, (int) Files.size(filePath) / BUFFER_SIZE + 1)
                    .map(i -> {
                        try {
                            if (is.available() > 0) {
                                var bodyBytes = ByteString.readFrom(is, 1024, BUFFER_SIZE);
                                if (bodyBytes.size() == 0) {
                                    return Optional.<Mgmt.LoadPartRequest>empty();
                                } else {
                                    var request = Mgmt.LoadPartRequest.newBuilder()
                                            .setName(name)
                                            .setBody(bodyBytes)
                                            .build();
                                    return Optional.of(request);
                                }
                            }
                        } catch (IOException e) {
                            System.err.println("Cannot check if data left in runnable file");
                        }
                        return Optional.<Mgmt.LoadPartRequest>empty();
                    })
                    .filter(Optional::isPresent)
                    .map(Optional::get);
            var reply = manager.get().load(multi)
                    .await()
                    .atMost(Duration.ofMinutes(2));
            if (reply.getSuccess()) {
                System.out.println("Successfully deployed file");
            } else {
                System.err.println("Error during deployment: " + reply.getErrorMessage());
            }
        }
        return 0;
    }
}
