#!/bin/sh

tmux new-session -d -s foo 'LD_LIBRARY_PATH=../../bin ./../../bin/c-example --topics /eth2/beacon_block/ssz,/eth2/beacon_attestation/ssz,/eth2/voluntary_exit/ssz,/eth2/proposer_slashing/ssz,/eth2/attester_slashing/ssz --debug-level trace'
tmux split-window -v -t 0 'sleep 2 && LD_LIBRARY_PATH=../../bin ./../../bin/c-example --topics /eth2/beacon_block/ssz,/eth2/beacon_attestation/ssz,/eth2/voluntary_exit/ssz,/eth2/proposer_slashing/ssz,/eth2/attester_slashing/ssz --boot-nodes $(cat ~/.mothra/network/enr.dat) --port 9001 --datadir /tmp/.mothra --debug-level trace'
tmux select-layout tile
tmux rename-window 'the dude abides'
tmux attach-session -d
