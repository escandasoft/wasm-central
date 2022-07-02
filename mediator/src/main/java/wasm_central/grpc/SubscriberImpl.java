package wasm_central.grpc;

import fn_proto.Fn;
import fn_proto.Subscriber;
import io.smallrye.mutiny.Multi;
import wasm_central.conf.FnHostRemote;

import javax.inject.Inject;

public class SubscriberImpl implements Subscriber {
    @Override
    public Multi<Fn.TopicResult> subscribe(Multi<Fn.Topic> request) {
        return Multi.createFrom().generator(() -> null, null);
    }
}
