package net.p2p;

import java.util.Objects;
import java.util.function.BiFunction;
import java.util.function.Function;

public class Mothra {
    public static final String MOTHRA = "mothra-jni";
    public static Function<String, Boolean> DiscoveryMessage;
    public static BiFunction<String, byte[], Boolean> ReceivedGossipMessage;
    public static QuadFunction<String, Integer, String, byte[], Boolean> ReceivedRPCMessage;
    public static native void Init();
    public static native void Start(String[] client_constants, String[] args);
    public static native void SendGossip(byte[] topic, byte[] message);
    public static native void SendRPC(byte[] method, int req_resp, byte[] peer, byte[] message);
    public static void DiscoveredPeer(byte[] peer) {
        DiscoveryMessage.apply(new String(peer));
    }
    public static void ReceiveGossip(byte[] topic, byte[] message) {
        ReceivedGossipMessage.apply(new String(topic), message);
    }
    public static void ReceiveRPC(byte[] method, int req_resp, byte[] peer, byte[] message) {
        ReceivedRPCMessage.apply(new String(method), req_resp, new String(peer), message);
    }
    static {
        try {
            System.loadLibrary ( MOTHRA ) ;
        } catch (UnsatisfiedLinkError e) {
          System.err.println("Native code library failed to load.\n" + e);
          e.printStackTrace();
          System.exit(1);
        }
    }

    @FunctionalInterface
    public interface QuadFunction<A,B,C,D,R> {
        R apply(A a, B b, C c, D d);
        default <V> QuadFunction<A, B, C, D, V> andThen(
                                    Function<? super R, ? extends V> after) {
            Objects.requireNonNull(after);
            return (A a, B b, C c, D d) -> after.apply(apply(a, b, c, d));
        }
    }
}
