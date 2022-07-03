package wasm;

import io.quarkus.grpc.GrpcClient;
import io.smallrye.mutiny.Uni;
import mgmt_proto.Manager;
import mgmt_proto.Mgmt;
import picocli.CommandLine;

import javax.enterprise.inject.Instance;
import java.time.Duration;

@CommandLine.Command(name = "function")
public class FunctionCommand {
    @GrpcClient
    Instance<Manager> manager;

    @CommandLine.Command(name = "list")
    public int list() {
        System.out.println("!! listing functions");
        Mgmt.ListRequest request = Mgmt.ListRequest.newBuilder().build();
        Uni<Mgmt.ListReply> reply = manager.get().list(request);
        reply.await()
                .atMost(Duration.ofMinutes(1))
                .getItemsList()
                .forEach(listReplyItem -> System.out.print(listReplyItem.getName() + "\t" + listReplyItem.getStatus()));
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
