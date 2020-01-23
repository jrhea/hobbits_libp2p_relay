#ifndef _MOTHRA_H_
#define _MOTHRA_H_

#ifdef __cplusplus
extern "C" {
#endif

extern void libp2p_start(char** args, int length);
extern void libp2p_send_gossip(unsigned char* topic_utf8, int topic_length, unsigned char* data, int data_length);
extern void libp2p_send_rpc_request(unsigned char* method_utf8, int method_length, unsigned char* peer_utf8, int peer_length, unsigned char* data, int data_length);
extern void libp2p_send_rpc_response(unsigned char* method_utf8, int method_length, unsigned char* peer_utf8, int peer_length, unsigned char* data, int data_length);

extern void libp2p_register_handlers(
   void (*discovered_peer_ptr)(const unsigned char* peer_utf8, int peer_length), 
   void (*receive_gossip_ptr)(const unsigned char* topic_utf8, int topic_length, unsigned char* data, int data_length), 
   void (*receive_rpc_ptr)(const unsigned char* method_utf8, int method_length, int req_resp, const unsigned char* peer_utf8, int peer_length, unsigned char* data, int data_length)
);
       
void discovered_peer(const unsigned char* peer_utf8, int peer_length);
void receive_gossip(const unsigned char* topic_utf8, int topic_length, unsigned char* data, int data_length);
void receive_rpc(const unsigned char* method_utf8, int method_length, int req_resp, const unsigned char* peer_utf8, int peer_length, unsigned char* data, int data_length);

#ifdef __cplusplus
}
#endif

#endif // _MOTHRA_H_