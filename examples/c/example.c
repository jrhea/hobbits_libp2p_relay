#ifdef _WIN64
#include <windows.h>
#else
#include <unistd.h>
#endif

#include <stdio.h>
#include <string.h>
#include "mothra-c.h"

#define sleep_seconds 5


void on_discovered_peer(const unsigned char* peer_utf8, int peer_length) {
    printf("C: discovered peer");
    printf(",peer=%.*s\n", peer_length, peer_utf8);
}

void on_receive_gossip(const unsigned char* topic_utf8, int topic_length, unsigned char* data, int data_length) {
    printf("C: received gossip");
    printf(",topic=%.*s", topic_length, topic_utf8);
    printf(",data=%.*s\n", data_length, data);
}

void on_receive_rpc(const unsigned char* method_utf8, int method_length, int req_resp, const unsigned char* peer_utf8, int peer_length, unsigned char* data, int data_length) {
    printf("C: received rpc %i", req_resp);
    printf(",method=%.*s", method_length, method_utf8);
    printf(",peer=%.*s", peer_length, peer_utf8);
    printf(",data=%.*s\n", data_length, data);
}

int main (int argc, char** argv) {
    printf("one\n");
    register_handlers(
        on_discovered_peer,
        on_receive_gossip,
        on_receive_rpc
    );
     printf("two\n");
    network_start(argv,argc);
     printf("three\n");
    while(1){
#ifdef _WIN64
        Sleep(sleep_seconds * 1000);
#else
        sleep(sleep_seconds);
#endif
        char* topic = "/eth2/beacon_block/ssz";
        int topic_length = (int)(strlen(topic));
        char* data = "Hello from C";
        int data_length = (int)(strlen(data));
        send_gossip((unsigned char*)topic, topic_length, (unsigned char*)data, data_length);
    }
}
