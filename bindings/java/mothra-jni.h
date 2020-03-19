#include <jni.h>
#ifndef _MOTHRA_JNI_H_
#define _MOTHRA_JNI_H_
#include "mothra.h"
#ifdef __cplusplus
extern "C" {
#endif
JNIEXPORT void JNICALL Java_net_p2p_Mothra_Init(JNIEnv*,jclass);
JNIEXPORT void JNICALL Java_net_p2p_Mothra_Start(JNIEnv *, jclass, jobjectArray);
JNIEXPORT void JNICALL Java_net_p2p_Mothra_SendGossip(JNIEnv *, jclass, jbyteArray, jbyteArray);
JNIEXPORT void JNICALL Java_net_p2p_Mothra_SendRPC (JNIEnv *, jclass, jbyteArray, jint, jbyteArray, jbyteArray);

void discovered_peer_callback(const unsigned char*, int);
void receive_gossip_callback(const unsigned char*, int, unsigned char*, int);
void receive_rpc_callback(const unsigned char*, int, int, const unsigned char*, int, unsigned char*, int);

static void detach(JNIEnv* );
#ifdef __cplusplus
}
#endif

#endif // _MOTHRA_JNI_H_
