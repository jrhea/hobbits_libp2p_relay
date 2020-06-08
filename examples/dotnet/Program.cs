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
using System.Text;
using System.Threading.Tasks;
using System.Runtime.InteropServices;
using System.Threading;
using Bindings;

namespace Example
{
    class Program
    {
        // To build, from root of mothra project, this will build the
        // C-bindings, then compile the dotnet DLL:
        //  make dotnet

        // Also make the C native client (for demo)
        //  make c

        // To run example program:
        //  dotnet bin/dotnet/Example.dll

        // To demo, open a second console and, run the C native client (on a different port):
        //  bin/example --boot-nodes $(cat ~/.mothra/network/enr.dat) --listen-address 127.0.0.1 --port 9001 --datadir /tmp/.mothra9001

        // Open a third console and run a third client (dotnet):
        //  dotnet bin/dotnet/Example.dll -- --boot-nodes $(cat ~/.mothra/network/enr.dat) --listen-address 127.0.0.1 --port 9002 --datadir /tmp/.mothra9002

        // You can also start C-native first, i.e.:
        //  bin/example
        //  dotnet bin/dotnet/Example.dll -- --boot-nodes $(cat ~/.mothra/network/enr.dat) --listen-address 127.0.0.1 --port 9001 --datadir /tmp/.mothra9001

        // Note: you can also run dotnet project directly (with c-bindings):
        //  make c
        //  dotnet run --project examples/dotnet

        // HACK: On Linux only working when debugger attached:
        // 1. build c-bindings: make c
        // 2. run C native in one console: bin/example
        // 3. open Example.csproj in Jetbrains Rider
        // 4. set program arguments in debug configuration:
        //     -- --boot-nodes enr:-Iu4QOcRj-KivlPmJ8FNyYGCV7Kkub3j8OzMwXCL-iZijl8kEg4nz2J3xTP5ENqMr5QgExjP9bzI7hOHZuDWhOjsPcUBgmlkgnY0gmlwhH8AAAGJc2VjcDI1NmsxoQKVrVQHZsUqntitqKx6o6cQBmwvA78SzeCb8jTLcHY_iYN0Y3CCIyiDdWRwgiMo --listen-address 127.0.0.1 --port 9001 --datadir /tmp/.artemis
        //     (this should match the enr of the running C native; yes, it starts with two dashes to separate program input)
        // 5. run in debug mode; it works fine (is stable)
        // 6. run without debugger: crashes on first message

        // To see the error:
        // gdb --args dotnet ../../bin/Example.dll  -- --boot-nodes enr:-Iu4QOcRj-KivlPmJ8FNyYGCV7Kkub3j8OzMwXCL-iZijl8kEg4nz2J3xTP5ENqMr5QgExjP9bzI7hOHZuDWhOjsPcUBgmlkgnY0gmlwhH8AAAGJc2VjcDI1NmsxoQKVrVQHZsUqntitqKx6o6cQBmwvA78SzeCb8jTLcHY_iYN0Y3CCIyiDdWRwgiMo --listen-address 127.0.0.1 --port 9001 --datadir /tmp/.artemis
        // then run, gets SIGSEGV, bt

        private static Handlers? s_handlers;
        
        private static GCHandle s_discoveredPeerHandle;
        private static GCHandle s_receiveGossipHandle;
        private static GCHandle s_receiveRpcHandle;

        private static Mothra.DiscoveredPeer? s_discoveredPeer;
        private static Mothra.ReceiveGossip? s_receiveGossip;
        private static Mothra.ReceiveRpc? s_receiveRpc;

        private static GCHandle s_args;
        
        static async Task Main(string[] args)
        {
            AppDomain.CurrentDomain.UnhandledException += CurrentDomainOnUnhandledException;

            Console.WriteLine("Press CTRL+C to exit");
            string[] clientConstants = new string[] {"dotnet-example","v0.1.0-unstable","dotnet-example/libp2p"};
            Start(clientConstants, args);
            
            while (true)
            {
                await Task.Delay(TimeSpan.FromSeconds(10));

                string message = string.Format("Hello libp2p from .NET {0:s}", DateTimeOffset.Now);
                Console.WriteLine($"dotnet: Sending {message}");

                byte[] data = Encoding.UTF8.GetBytes(message);

                SendGossip("/mothra/topic1", data);
            }
        }

        private static void CurrentDomainOnUnhandledException(object sender, UnhandledExceptionEventArgs e)
        {
            Console.WriteLine("Unhandled: {0}", e);
        }

        public static unsafe void Start(string[] clientConstants, string[] args)
        {
            s_handlers = new Handlers();
            
            // MothraInterop.DiscoveredPeer discoveredPeer = new MothraInterop.DiscoveredPeer(OnDiscoveredPeer);
            // MothraInterop.ReceiveGossip receiveGossip = new MothraInterop.ReceiveGossip(OnReceiveGossip);
            // MothraInterop.ReceiveRpc receiveRpc = new MothraInterop.ReceiveRpc(OnReceiveRpc);
            s_discoveredPeer = new Mothra.DiscoveredPeer(s_handlers.OnDiscoveredPeer);
            s_receiveGossip = new Mothra.ReceiveGossip(s_handlers.OnReceiveGossip);
            s_receiveRpc = new Mothra.ReceiveRpc(s_handlers.OnReceiveRpc);

            // as delegates
            // s_discoveredPeerHandle = GCHandle.Alloc(discoveredPeer);
            // s_receiveGossipHandle = GCHandle.Alloc(receiveGossip);            
            // s_receiveRpcHandle = GCHandle.Alloc(receiveRpc);
            s_discoveredPeerHandle = GCHandle.Alloc(s_discoveredPeer);
            s_receiveGossipHandle = GCHandle.Alloc(s_receiveGossip);
            s_receiveRpcHandle = GCHandle.Alloc(s_receiveRpc);

            // as pointers
            // IntPtr discoveredPeerPtr = Marshal.GetFunctionPointerForDelegate(discoveredPeer);
            // IntPtr receiveGossipPtr = Marshal.GetFunctionPointerForDelegate(receiveGossip);
            // IntPtr receiveRpcPtr = Marshal.GetFunctionPointerForDelegate(receiveRpc);
            // s_discoveredPeerHandle = GCHandle.Alloc(discoveredPeerPtr, GCHandleType.Pinned);
            // s_receiveGossipHandle = GCHandle.Alloc(receiveGossipPtr, GCHandleType.Pinned);            
            // s_receiveRpcHandle = GCHandle.Alloc(receiveRpcPtr, GCHandleType.Pinned);

            // as static field pointers
            // s_discoveredPeerPtr = Marshal.GetFunctionPointerForDelegate(discoveredPeer);
            // s_receiveGossipPtr = Marshal.GetFunctionPointerForDelegate(receiveGossip);
            // s_receiveRpcPtr = Marshal.GetFunctionPointerForDelegate(receiveRpc);
            // s_discoveredPeerHandle = GCHandle.Alloc(s_discoveredPeerPtr, GCHandleType.Pinned);
            // s_receiveGossipHandle = GCHandle.Alloc(s_receiveGossipPtr, GCHandleType.Pinned);            
            // s_receiveRpcHandle = GCHandle.Alloc(s_receiveRpcPtr, GCHandleType.Pinned);

            //MothraInterop.RegisterHandlers(discoveredPeer, receiveGossip, receiveRpc);
            Mothra.RegisterHandlers(s_discoveredPeer, s_receiveGossip, s_receiveRpc);
            Thread.Sleep(1000);
            //MothraInterop.RegisterHandlers(discoveredPeerPtr, receiveGossipPtr, receiveRpcPtr);
            // MothraInterop.RegisterHandlers(s_discoveredPeerPtr, s_receiveGossipPtr, s_receiveRpcPtr);

            s_args = GCHandle.Alloc(args);
            Mothra.Start(clientConstants, clientConstants.Length, args, args.Length);
        }

        public static unsafe void SendGossip(string topic, ReadOnlySpan<byte> data)
        {
            byte[] topicUtf8 = Encoding.UTF8.GetBytes(topic);
            fixed (byte* topicUtf8Ptr = topicUtf8)
            fixed (byte* dataPtr = data)
            {
                Mothra.SendGossip(topicUtf8Ptr, topicUtf8.Length, dataPtr, data.Length);
            }
        }
    }

    public class Handlers
    {
        public unsafe void OnDiscoveredPeer(byte* peerUtf8, int peerLength)
        {
            Console.Write("dotnet: peer");
            string peer = new String((sbyte*)peerUtf8, 0, peerLength, Encoding.UTF8);
            Console.WriteLine($" discovered {peer}");
        }

        public unsafe void OnReceiveGossip(byte* messageIdUtf8, int messageIdLength, byte* peerIdUtf8, int peerIdLength, byte* topicUtf8, int topicLength, byte* data, int dataLength)
        {
            Console.Write("dotnet: receive");
            string messageId = new String((sbyte*)messageIdUtf8, 0, messageIdLength, Encoding.UTF8);
            string peerId = new String((sbyte*)peerIdUtf8, 0, peerIdLength, Encoding.UTF8);
            string topic = new String((sbyte*)topicUtf8, 0, topicLength, Encoding.UTF8);
            string dataString = new String((sbyte*)data, 0, dataLength, Encoding.UTF8);
            Console.WriteLine($"messageId={messageId},peerId={peerId},gossip={topic},data={dataString}");
        }

        public unsafe void OnReceiveRpc(byte* methodUtf8, int methodLength, int requestResponseFlag, byte* peerUtf8,
            int peerLength, byte* data, int dataLength)
        {
            // Nothing
        }
    }
}
