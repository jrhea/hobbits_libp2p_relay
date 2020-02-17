import java.util.List;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Scanner;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import net.p2p.mothra;

public class example {
    public static void main(String[] args) throws InterruptedException {
        List<String> argList = new ArrayList<String>(Arrays.asList(args));
        argList.add(0,"./example");
        final String[] processed_args = argList.toArray(new String[0]);
        Runnable run = () -> {
            mothra.Init();
            mothra.Start(processed_args);
            mothra.DiscoveryMessage = example::printDiscoveryMessage;
            mothra.ReceivedGossipMessage = example::printGossipMessage;
            mothra.ReceivedRPCMessage = example::printRPCMessage;
        };
        Executors.newSingleThreadExecutor().execute(run);
        Scanner scanner = new Scanner(System.in);
        while(true){
            System.out.print("Select RPC or GOSSIP: ");
            String messageType = scanner.next();
            if(messageType.equals("GOSSIP")){
                System.out.print("Enter a message to GOSSIP: ");
                String message = scanner.next();
                mothra.SendGossip("/eth2/beacon_block/ssz".getBytes(),message.getBytes());
            } else if(messageType.equals("RPC")){
                System.out.print("Enter Req/Resp (0/1): ");
                int req_resp = Integer.parseInt(scanner.next());
                System.out.print("Enter a Peer: ");
                String peer = scanner.next();
                System.out.print("Enter a message: ");
                String message = scanner.next();
                mothra.SendRPC("HELLO".getBytes(),req_resp,peer.getBytes(),message.getBytes());
            }
        }

    }

    public static Boolean printDiscoveryMessage(String peer){
        System.out.println("Java: Discovered a new peer " + peer);
        return true;
    }

    public static Boolean printGossipMessage(String topic, byte[] message){
        System.out.println("Java: received a gossip message. " + topic + ":" + new String(message));
        return true;
    }

    public static Boolean printRPCMessage(String method, Integer req_resp, String peer, byte[] message){
        System.out.println("Java: rpc method  " + method + ":" + req_resp + " was invoked by peer: " + peer + " with message: " + new String(message));
        return true;
    }
}