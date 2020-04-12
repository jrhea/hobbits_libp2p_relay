#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <assert.h>
#include "mothra-jni.h"

static JavaVM *jvm;

JNIEXPORT void JNICALL Java_p2p_mothra_Mothra_Init(JNIEnv* jenv, jclass jcls)
{
   jint rs = (*jenv)->GetJavaVM(jenv, &jvm);
   assert (rs == JNI_OK);
   register_handlers(discovered_peer_callback,receive_gossip_callback,receive_rpc_callback);
}

JNIEXPORT void JNICALL Java_p2p_mothra_Mothra_Start(JNIEnv *jenv, jclass jcls, jobjectArray jclient_constants, jobjectArray jargs){
    // allocate memory
    int num_client_constants = (*jenv)->GetArrayLength(jenv, jclient_constants);
    int num_args = (*jenv)->GetArrayLength(jenv, jargs);

    // copy arrays
    char **client_constants = copy_array(jenv, jclient_constants, num_client_constants);
    char **args = copy_array(jenv, jargs, num_args);
    if(!client_constants || !args) return;

    network_start(client_constants, num_client_constants, args, num_args);

    // free memory
    for (int i=0; i<num_client_constants; i++) {
        free(client_constants[i]);
    }
    free(client_constants);
    for (int i=0; i<num_args; i++) {
        free(args[i]);
    }
    free(args);
}

JNIEXPORT void JNICALL Java_p2p_mothra_Mothra_SendGossip(JNIEnv *jenv, jclass jcls, jbyteArray jtopic, jbyteArray jdata){
    int data_length = (*jenv)->GetArrayLength(jenv, jdata);
    int topic_length = (*jenv)->GetArrayLength(jenv, jtopic);
    unsigned char *topic = (unsigned char *) 0 ;
    unsigned char *data = (unsigned char *) 0 ;
    jboolean isCopy = JNI_TRUE;
    if (jtopic) {
        topic = (unsigned char *)(*jenv)->GetByteArrayElements(jenv,jtopic,&isCopy);
        if (!topic) return ;
    }
    if (jdata) {
        data = (unsigned char *)(*jenv)->GetByteArrayElements(jenv,jdata,&isCopy);
        if (!data) return ;
    }
    send_gossip(topic,topic_length,data,data_length);
    if (topic) (*jenv)->ReleaseByteArrayElements(jenv, jtopic, (jbyte *)topic, 0);
    if (data) (*jenv)->ReleaseByteArrayElements(jenv, jdata, (jbyte *)data, 0);
}

JNIEXPORT void JNICALL Java_p2p_mothra_Mothra_SendRPC(JNIEnv *jenv, jclass jcls, jbyteArray jmethod, jint jreq_resp, jbyteArray jpeer, jbyteArray jdata){
    int data_length = (*jenv)->GetArrayLength(jenv, jdata);
    int method_length = (*jenv)->GetArrayLength(jenv, jmethod);
    int peer_length = (*jenv)->GetArrayLength(jenv, jpeer);
    unsigned char *data = (unsigned char *) 0 ;
    unsigned char *method = (unsigned char *) 0 ;
    unsigned char *peer = (unsigned char *) 0 ;
    jboolean isCopy = JNI_TRUE;
    if (jdata) {
        data = (unsigned char *)(*jenv)->GetByteArrayElements(jenv,jdata,&isCopy);
        if (!data) return ;
    }
    if (jpeer) {
        peer = (unsigned char *)(*jenv)->GetByteArrayElements(jenv,jpeer,&isCopy);
        if (!peer) return ;
    }
    if (jmethod) {
        method = (unsigned char *)(*jenv)->GetByteArrayElements(jenv,jmethod,&isCopy);
        if (!method) return ;
    }
    if (jreq_resp == 0){
        send_rpc_request(method,method_length,peer,peer_length,data,data_length);
    } else if (jreq_resp == 1){
        send_rpc_response(method,method_length,peer,peer_length,data,data_length);
    }
    if (data) (*jenv)->ReleaseByteArrayElements(jenv, jdata, (jbyte *)data, 0);
    if (peer) (*jenv)->ReleaseByteArrayElements(jenv, jpeer, (jbyte *)peer, 0);
    if (method) (*jenv)->ReleaseByteArrayElements(jenv, jmethod, (jbyte *)method, 0);
}

void discovered_peer_callback(const unsigned char* peer, int peer_length) {
    JNIEnv *jenv;
    jint rs = (*jvm)->AttachCurrentThread(jvm, (void**)&jenv, NULL);
    assert (rs == JNI_OK);
    if(jenv != NULL) {
        jclass mothra_class;
        jmethodID discoveredpeer_method;
        jbyteArray jpeer;
          mothra_class = (*jenv)->FindClass(jenv, "p2p/mothra/Mothra");
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

void receive_gossip_callback(const unsigned char* topic, int topic_length, unsigned char* data, int data_length) {
    JNIEnv *jenv;
    jint rs = (*jvm)->AttachCurrentThread(jvm, (void**)&jenv, NULL);
    assert (rs == JNI_OK);
    if(jenv != NULL) {
        jclass mothra_class;
        jmethodID receivegossip_method;
        jbyteArray jtopic;
        jbyteArray jdata;
        mothra_class = (*jenv)->FindClass(jenv, "p2p/mothra/Mothra");
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

void receive_rpc_callback(const unsigned char* method, int method_length, int req_resp, const unsigned char* peer, int peer_length, unsigned char* data, int data_length) {
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
        mothra_class = (*jenv)->FindClass(jenv, "p2p/mothra/Mothra");
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

static char** copy_array(JNIEnv *jenv, jobjectArray src, int num) {
    char **result = (char **) malloc(num * sizeof(char *));
    for (int i=0; i<num; i++) {
        jstring jitem = (jstring) ((*jenv)->GetObjectArrayElement(jenv, src, i));
        const char *item = (*jenv)->GetStringUTFChars(jenv, jitem, 0);
        result[i] = (char*) malloc(strlen(item)*sizeof(char*));
        strcpy(result[i],item);
        (*jenv)->ReleaseStringUTFChars(jenv, jitem, (const char *)item);
    }
    return result;
}

static void detach(JNIEnv* jenv){
    if((*jenv)->ExceptionOccurred(jenv)) {
        (*jenv)->ExceptionDescribe(jenv);
    }
    (*jvm)->DetachCurrentThread(jvm);
}