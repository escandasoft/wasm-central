package wasm_central;

import wasm_central.grpc.SubscriberImpl;

import javax.enterprise.context.ApplicationScoped;
import javax.enterprise.inject.Produces;

@ApplicationScoped
public class GrpcConfiguration {
    

    @Produces
    @ApplicationScoped
    SubscriberImpl subscriber() {
        return new SubscriberImpl();
    }
}
