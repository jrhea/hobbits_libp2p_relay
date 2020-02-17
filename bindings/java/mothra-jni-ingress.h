#include <jni.h>
#ifndef _MOTHRA_JNI_INGRESS_H_
#define _MOTHRA_JNI_INGRESS_H_

#ifdef __cplusplus
extern "C" {
#endif

JNIEXPORT void JNICALL Java_net_p2p_mothra_Init(JNIEnv*,jclass);

void ingress_register_handlers(
   void (*discovered_peer_ptr)(const unsigned char* peer_utf8, int peer_length), 
   void (*receive_gossip_ptr)(const unsigned char* topic_utf8, int topic_length, unsigned char* data, int data_length), 
   void (*receive_rpc_ptr)(const unsigned char* method_utf8, int method_length, int req_resp, const unsigned char* peer_utf8, int peer_length, unsigned char* data, int data_length)
);

void discovered_peer(const unsigned char*, int);
void receive_gossip(const unsigned char*, int, unsigned char*, int);
void receive_rpc(const unsigned char*, int, int, const unsigned char*, int, unsigned char*, int);

void detach(JNIEnv* );

#ifdef __cplusplus
}
#endif

#endif // _MOTHRA_JNI_INGRESS_H_
