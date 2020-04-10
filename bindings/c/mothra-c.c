#include <stdio.h>
#include "mothra-c.h"

void (*s_discovered_peer_ptr)(const unsigned char* peer, int peer_length);
void (*s_receive_gossip_ptr)(const unsigned char* topic, int topic_length, unsigned char* data, int data_length);
void (*s_receive_rpc_ptr)(const unsigned char* method, int method_length, int req_resp, const unsigned char* peer, int peer_length, unsigned char* data, int data_length);

void discovered_peer(const unsigned char* peer, int peer_length) {
    s_discovered_peer_ptr(peer, peer_length);
}

void receive_gossip(const unsigned char* topic, int topic_length, unsigned char* data, int data_length) {
    s_receive_gossip_ptr(topic, topic_length, data, data_length);
}

void receive_rpc(const unsigned char* method, int method_length, int req_resp, const unsigned char* peer, int peer_length, unsigned char* data, int data_length) {
    s_receive_rpc_ptr(method, method_length, req_resp, peer, peer_length, data, data_length);
}
