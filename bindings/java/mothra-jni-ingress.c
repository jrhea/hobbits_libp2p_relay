#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <assert.h>
#include "mothra-jni-ingress.h"

static JavaVM *jvm;

JNIEXPORT void JNICALL Java_net_p2p_mothra_Init(JNIEnv* jenv, jclass jcls)
{
   jint rs = (*jenv)->GetJavaVM(jenv, &jvm);
   assert (rs == JNI_OK);
}

void (*s_discovered_peer_ptr)(const unsigned char* peer_utf8, int peer_length);
void (*s_receive_gossip_ptr)(const unsigned char* topic_utf8, int topic_length, unsigned char* data, int data_length);
void (*s_receive_rpc_ptr)(const unsigned char* method_utf8, int method_length, int req_resp, const unsigned char* peer_utf8, int peer_length, unsigned char* data, int data_length);

void ingress_register_handlers(
    void (*discovered_peer_ptr)(const unsigned char* peer_utf8, int peer_length), 
    void (*receive_gossip_ptr)(const unsigned char* topic_utf8, int topic_length, unsigned char* data, int data_length), 
    void (*receive_rpc_ptr)(const unsigned char* method_utf8, int method_length, int req_resp, const unsigned char* peer_utf8, int peer_length, unsigned char* data, int data_length)
) {
    s_discovered_peer_ptr = discovered_peer_ptr;
    s_receive_gossip_ptr = receive_gossip_ptr;
    s_receive_rpc_ptr = receive_rpc_ptr;
}

void discovered_peer(const unsigned char* peer, int peer_length) {
    JNIEnv *jenv;
    jint rs = (*jvm)->AttachCurrentThread(jvm, (void**)&jenv, NULL);
    assert (rs == JNI_OK);
    if(jenv != NULL) {
        jclass mothra_class;
        jmethodID discoveredpeer_method;
        jbyteArray jpeer;
          mothra_class = (*jenv)->FindClass(jenv, "net/p2p/mothra");
        if(!mothra_class){
            detach(jenv);
        }
        //Put the native unsigned chars in the java byte array
        jpeer = (*jenv)->NewByteArray(jenv, peer_length);
        (*jenv)->SetByteArrayRegion(jenv, jpeer, 0, peer_length, (jbyte *)peer);
        if(!jpeer){
            detach(jenv);
        }
        discoveredpeer_method = (*jenv)->GetStaticMethodID(jenv, mothra_class, "DiscoveredPeer", "([B)V");
        if(!discoveredpeer_method){
            printf("JNI Error: GetStaticMethodID was unable to find method: DiscoveredPeer with signature: ([B)V\n");
            detach(jenv);
        }
        (*jenv)->CallStaticVoidMethod(jenv, mothra_class, discoveredpeer_method, jpeer);
    }
}

void receive_gossip(const unsigned char* topic, int topic_length, unsigned char* data, int data_length) {
    JNIEnv *jenv;
    jint rs = (*jvm)->AttachCurrentThread(jvm, (void**)&jenv, NULL);
    assert (rs == JNI_OK);
    if(jenv != NULL) {
        jclass mothra_class;
        jmethodID receivegossip_method;
        jbyteArray jtopic;
        jbyteArray jdata;
        mothra_class = (*jenv)->FindClass(jenv, "net/p2p/mothra");
        if(!mothra_class){
            detach(jenv);
        }
        //Put the native unsigned chars in the java byte array
        jtopic = (*jenv)->NewByteArray(jenv, topic_length);
        jdata = (*jenv)->NewByteArray(jenv, data_length);
        (*jenv)->SetByteArrayRegion(jenv, jdata, 0, data_length, (jbyte *)data);
        (*jenv)->SetByteArrayRegion(jenv, jtopic, 0, topic_length, (jbyte *)topic);
        if(!jdata || !jtopic){
            detach(jenv);
        }
        receivegossip_method = (*jenv)->GetStaticMethodID(jenv, mothra_class, "ReceiveGossip", "([B[B)V");
        if(!receivegossip_method){
            printf("JNI Error: GetStaticMethodID was unable to find method: ReceiveGossip with signature: ([B[B)V\n");
            detach(jenv);
        }
        (*jenv)->CallStaticVoidMethod(jenv, mothra_class, receivegossip_method, jtopic, jdata);
    }
}

void receive_rpc(const unsigned char* method, int method_length, int req_resp, const unsigned char* peer, int peer_length, unsigned char* data, int data_length) {
    JNIEnv *jenv;
    jint rs = (*jvm)->AttachCurrentThread(jvm, (void**)&jenv, NULL);
    assert (rs == JNI_OK);
    if(jenv != NULL) {
        jclass mothra_class;
        jmethodID receiverpc_method;
        jbyteArray jmethod;
        jint jreq_resp;
        jbyteArray jpeer;
        jbyteArray jdata;
        mothra_class = (*jenv)->FindClass(jenv, "net/p2p/mothra");
        if(!mothra_class){
            detach(jenv);
        }
        //Put the native unsigned chars in the java byte array
        jmethod = (*jenv)->NewByteArray(jenv, method_length);
        jreq_resp = req_resp;
        jpeer = (*jenv)->NewByteArray(jenv, peer_length);
        jdata = (*jenv)->NewByteArray(jenv, data_length);
        (*jenv)->SetByteArrayRegion(jenv, jmethod, 0, method_length, (jbyte *)method);
        (*jenv)->SetByteArrayRegion(jenv, jpeer, 0, peer_length, (jbyte *)peer);
        (*jenv)->SetByteArrayRegion(jenv, jdata, 0, data_length, (jbyte *)data);
        if(!jdata || !jpeer|| !jmethod){
            detach(jenv);
        }
        receiverpc_method = (*jenv)->GetStaticMethodID(jenv, mothra_class, "ReceiveRPC", "([BI[B[B)V");
        if(!receiverpc_method){
            printf("JNI Error: GetStaticMethodID was unable to find method: ReceiveRPC with signature: ([BI[B[B)V\n");
            detach(jenv);
        }
        (*jenv)->CallStaticVoidMethod(jenv, mothra_class, receiverpc_method, jmethod, jreq_resp, jpeer, jdata);
    }
}

void detach(JNIEnv* jenv){
    if((*jenv)->ExceptionOccurred(jenv)) {
        (*jenv)->ExceptionDescribe(jenv);
    }
    (*jvm)->DetachCurrentThread(jvm);
}
