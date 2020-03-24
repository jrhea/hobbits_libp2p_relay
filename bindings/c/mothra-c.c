#include <stdio.h>
#include "mothra-c.h"

void (*s_discovered_peer_ptr)(const unsigned char* peer_utf8, int peer_length);
void (*s_receive_gossip_ptr)(const unsigned char* topic_utf8, int topic_length, unsigned char* data, int data_length);
void (*s_receive_rpc_ptr)(const unsigned char* method_utf8, int method_length, int req_resp, const unsigned char* peer_utf8, int peer_length, unsigned char* data, int data_length);

void discovered_peer(const unsigned char* peer_utf8, int peer_length) {
    //printf("bind: peer=%.*s\n", peer_length, peer_utf8);
    s_discovered_peer_ptr(peer_utf8, peer_length);
}

void receive_gossip(const unsigned char* topic_utf8, int topic_length, unsigned char* data, int data_length) {
    //printf("bind: gossip=%.*s\n", topic_length, topic_utf8);
    s_receive_gossip_ptr(topic_utf8, topic_length, data, data_length);
}

void receive_rpc(const unsigned char* method_utf8, int method_length, int req_resp, const unsigned char* peer_utf8, int peer_length, unsigned char* data, int data_length) {
    s_receive_rpc_ptr(method_utf8, method_length, req_resp, peer_utf8, peer_length, data, data_length);
}
