#!/bin/sh

tmux new-session -d -s foo 'cd ./../../bin && java Example --topics /eth2/beacon_block/ssz,/eth2/beacon_attestation/ssz,/eth2/voluntary_exit/ssz,/eth2/proposer_slashing/ssz,/eth2/attester_slashing/ssz'
tmux split-window -v -t 0 'cd ./../../bin && java Example --topics /eth2/beacon_block/ssz,/eth2/beacon_attestation/ssz,/eth2/voluntary_exit/ssz,/eth2/proposer_slashing/ssz,/eth2/attester_slashing/ssz --boot-nodes $(cat ~/.mothra/network/enr.dat) --listen-address 127.0.0.1 --port 9001 --datadir /tmp/.mothra'
tmux select-layout tile
tmux rename-window 'the dude abides'
tmux attach-session -d
