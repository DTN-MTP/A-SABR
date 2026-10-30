# Each node carries a DelayHeuristicManager<NoManagement>:
# node <id> <name> [<delay to node 0>, <delay to node 1>, ...]
# Each entry is a lower bound on the delay from this node to that node.
node 0 source [0, 10, 1, 2]
node 1 slow_link [10, 0, 11, 10]
node 2 fast_link [1, 11, 0, 1]
node 3 destination [2, 10, 1, 0]

# contact <from> <to> <start> <end> <rate> <delay>
# Path through node 1: long propagation delays, but always available.
contact 0 1 0 100 1 10
contact 1 3 0 100 1 10
# Path through node 2: short delays, but node 2 only reaches the destination from t=50.
contact 0 2 0 100 1 1
contact 2 3 50 100 1 1
