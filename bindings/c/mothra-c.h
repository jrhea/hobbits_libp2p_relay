#ifndef _MOTHRA_C_H_
#define _MOTHRA_C_H_

#ifdef _WIN64
   #define EXPORT __declspec(dllexport)
   #define IMPORT __declspec(dllimport)
#else
   #define EXPORT __attribute__ ((visibility ("default")))
   #define IMPORT
#endif

#ifdef __cplusplus
extern "C" {
#endif

EXPORT void libp2p_start(char** args, int length);
EXPORT void libp2p_send_gossip(unsigned char* topic_utf8, int topic_length, unsigned char* data, int data_length);
EXPORT void libp2p_send_rpc_request(unsigned char* method_utf8, int method_length, unsigned char* peer_utf8, int peer_length, unsigned char* data, int data_length);
EXPORT void libp2p_send_rpc_response(unsigned char* method_utf8, int method_length, unsigned char* peer_utf8, int peer_length, unsigned char* data, int data_length);

EXPORT void libp2p_register_handlers(
   void (*discovered_peer_ptr)(const unsigned char* peer_utf8, int peer_length), 
   void (*receive_gossip_ptr)(const unsigned char* topic_utf8, int topic_length, unsigned char* data, int data_length), 
   void (*receive_rpc_ptr)(const unsigned char* method_utf8, int method_length, int req_resp, const unsigned char* peer_utf8, int peer_length, unsigned char* data, int data_length)
);
       
// Events functions called by Core
EXPORT void discovered_peer(const unsigned char* peer_utf8, int peer_length);
EXPORT void receive_gossip(const unsigned char* topic_utf8, int topic_length, unsigned char* data, int data_length);
EXPORT void receive_rpc(const unsigned char* method_utf8, int method_length, int req_resp, const unsigned char* peer_utf8, int peer_length, unsigned char* data, int data_length);

#ifdef __cplusplus
}
#endif

#endif // _MOTHRA_C_H_