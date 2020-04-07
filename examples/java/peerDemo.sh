#!/bin/sh

tmux new-session -d -s foo 'cd ./../../bin && java Example --topics /eth2/beacon_block/ssz,/eth2/beacon_attestation/ssz,/eth2/voluntary_exit/ssz,/eth2/proposer_slashing/ssz,/eth2/attester_slashing/ssz --debug-level trace'
tmux split-window -v -t 0 'sleep 2 && cd ./../../bin && java Example --topics /eth2/beacon_block/ssz,/eth2/beacon_attestation/ssz,/eth2/voluntary_exit/ssz,/eth2/proposer_slashing/ssz,/eth2/attester_slashing/ssz --boot-nodes $(cat ~/.mothra/network/enr.dat) --port 9001 --datadir /tmp/.mothra --debug-level trace'
tmux select-layout tile
tmux rename-window 'the dude abides'
tmux attach-session -d
