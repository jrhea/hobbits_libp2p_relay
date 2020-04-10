#ifdef _WIN64
#include <windows.h>
#else
#include <unistd.h>
#endif

#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#include "mothra-c.h"

#define sleep_seconds 5
#define LEN(x)  (sizeof(x) / sizeof((x)[0]))


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

    char* client_constants[3] = {
        "c-example",
        "v0.1.0-unstable",
        "c-example/libp2p"
    };
    register_handlers(
        on_discovered_peer,
        on_receive_gossip,
        on_receive_rpc
    );
    network_start((char**)client_constants,LEN(client_constants),argv,argc);
    srand(time(NULL));
    while(1){
#ifdef _WIN64
        Sleep(sleep_seconds * 1000);
#else
        sleep(sleep_seconds);
#endif
        char* topic = "/eth2/beacon_block/ssz";
        int topic_length = (int)(strlen(topic));
        char r[3], data[50];
        sprintf(r, "%d",rand()%99);
        strcpy(data,"Hello from C.  Random number: ");
        strncat(data,r,20);
        int data_length = (int)(strlen(data));
        send_gossip((unsigned char*)topic, topic_length, (unsigned char*)data, data_length);
    }

}
