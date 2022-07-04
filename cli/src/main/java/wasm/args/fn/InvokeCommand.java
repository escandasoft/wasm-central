package wasm.args.fn;

import com.google.protobuf.ByteString;
import fn_proto.Executor;
import fn_proto.Fn;
import io.quarkus.grpc.GrpcClient;
import picocli.CommandLine;

import javax.enterprise.inject.Instance;
import java.io.FileInputStream;
import java.nio.file.Paths;
import java.time.Duration;
import java.util.concurrent.Callable;

@CommandLine.Command(name = "invoke")
public class InvokeCommand implements Callable<Integer> {
    @CommandLine.Option(names = {"-n", "--name"}, description = "Function name", required = true)
    String name;

    @CommandLine.Option(names = {"-p", "--payload"}, description = "Payload path", required = true)
    String payload;

    @GrpcClient
    Instance<Executor> executor;

    @Override
    public Integer call() throws Exception {
        System.out.println("!! invoking '" + name + "' with payload at " + payload);
        var payloadPath = Paths.get(payload);
        var file = payloadPath.toFile();
        try (var fis = new FileInputStream(file)) {
            var reply = executor.get().execute(Fn.ExecuteRequest.newBuilder()
                            .setName(name)
                            .setBody(ByteString.readFrom(fis))
                            .build())
                    .await()
                    .atMost(Duration.ofMinutes(2));
            System.out.println("Response code => " + reply.getCode());
            System.out.println("Response => " + reply.getBody());
        }
        return 0;
    }
}
