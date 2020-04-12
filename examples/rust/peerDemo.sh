#!/bin/sh

tmux new-session -d -s foo './../../bin/rust-example mothra --topics /mothra/topic1,/mothra/topic2 --debug-level trace'
tmux split-window -v -t 0 'sleep 2 && ./../../bin/rust-example mothra --topics /mothra/topic1,/mothra/topic2  --boot-nodes $(cat ~/.mothra/network/enr.dat) --port 9001 --datadir /tmp/.mothra --debug-level trace'
tmux select-layout tile
tmux rename-window 'the dude abides'
tmux attach-session -d
