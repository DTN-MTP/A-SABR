node 0 source
node 1 intermediary_1
node 2 intermediary_2
node 3 destination

# Format: contact [from] [to] [start_time] [end_time] [evl/eto/qd] [data_rate] [delay] [mav_p0] [mav_p1] [mav_p2]
contact 0 1 0 100 eto 10 1 10 10 10

contact 0 2 0 100 eto 10 1 10 10 10

contact 1 3 0 100 qd 10 1 10 10 10

contact 2 3 0 100 qd 10 1 10 10 10
