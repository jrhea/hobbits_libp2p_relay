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

EXPORT void network_start(char**, int, char**, int);
EXPORT void send_gossip(unsigned char*, int, unsigned char*, int);
EXPORT void send_rpc_request(unsigned char*, int, unsigned char*, int, unsigned char*, int);
EXPORT void send_rpc_response(unsigned char*, int, unsigned char*, int, unsigned char*, int);

EXPORT void register_handlers(
   void (*discovered_peer_ptr)(const unsigned char*, int), 
   void (*receive_gossip_ptr)(const unsigned char*, int, const unsigned char*, int, const unsigned char*, int, unsigned char*, int), 
   void (*receive_rpc_ptr)(const unsigned char*, int, int, const unsigned char*, int, unsigned char*, int)
);
       
// Events functions called by Core
EXPORT void discovered_peer(const unsigned char*, int);
EXPORT void receive_gossip(const unsigned char*, int, const unsigned char*, int, const unsigned char*, int, unsigned char*, int);
EXPORT void receive_rpc(const unsigned char*, int, int, const unsigned char*, int, unsigned char*, int);

#ifdef __cplusplus
}
#endif

#endif // _MOTHRA_C_H_