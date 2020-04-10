// Copyright 2020 Sly Gryphon
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

using System;
using System.Runtime.InteropServices;

namespace Bindings
{
    public static class Mothra
    {
        // mothra.dll on Windows, libmothra.so on Linux, libmotha.dylib on OSX
        private const string DllName = "libmothra";
        
        [DllImport(DllName, EntryPoint = "network_start", CallingConvention = CallingConvention.Cdecl)]
        public static extern unsafe void Start([In, Out] string[] clientConstants, int numClientConstants, [In, Out] string[] args, int numArgs);

        [DllImport(DllName, EntryPoint = "send_gossip", CallingConvention = CallingConvention.Cdecl)]
        public static extern unsafe void SendGossip(byte* topicUtf8, int topicLength, byte* data, int dataLength);

        [DllImport(DllName, EntryPoint = "send_rpc_request", CallingConvention = CallingConvention.Cdecl)]
        public static extern unsafe void SendRequest(byte* methodUtf8, int methodLength, byte* peerUtf8, int peerLength, byte* data, int dataLength);

        [DllImport(DllName, EntryPoint = "send_rpc_response", CallingConvention = CallingConvention.Cdecl)]
        public static extern unsafe void SendResponse(byte* methodUtf8, int methodLength, byte* peerUtf8, int peerLength, byte* data, int dataLength);

        [DllImport(DllName, EntryPoint = "register_handlers", CallingConvention = CallingConvention.Cdecl)]
        public static extern unsafe void RegisterHandlers(DiscoveredPeer discoveredPeer, ReceiveGossip receiveGossip, ReceiveRpc receiveRpc);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public unsafe delegate void DiscoveredPeer(byte* peerUtf8, int peerLength);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public unsafe delegate void ReceiveGossip(byte* topicUtf8, int topicLength, byte* data, int dataLength);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public unsafe delegate void ReceiveRpc(byte* methodUtf8, int methodLength, int requestResponseFlag, byte* peerUtf8, int peerLength, byte* data, int dataLength);
    }
}