package wasm.args.fn;

import io.quarkus.grpc.GrpcClient;
import mgmt_proto.Manager;
import mgmt_proto.Mgmt;
import picocli.CommandLine;

import javax.enterprise.inject.Instance;
import java.time.Duration;
import java.util.concurrent.Callable;

@CommandLine.Command(name = "list")
public class ListCommand implements Callable<Integer> {
    @GrpcClient
    Instance<Manager> manager;

    @Override
    public Integer call() throws Exception {
        System.out.println("!! listing functions");
        var request = Mgmt.ListRequest.newBuilder().build();
        var reply = manager.get().list(request);
        reply.await()
                .atMost(Duration.ofMinutes(1))
                .getItemsList()
                .forEach(listReplyItem -> System.out.print(listReplyItem.getName() + "\t" + listReplyItem.getStatus()));
        return 0;
    }
}
