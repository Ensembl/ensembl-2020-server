import math

steps = 1000

def prop(pos,leaf_start,leaf_end):
    return float(pos-leaf_start)/float(leaf_end-leaf_start)

def shimmer(starts,lens,senses,leaf_start,leaf_end):
    (out_starts, out_lens, out_senses) = ([],[],[])
    step_bp = (leaf_end-leaf_start) / steps
    # make buckets
    buckets_end = [False] * steps
    buckets_num = [0] * steps
    for i in range(0,len(starts)):
        start = math.floor(prop(starts[i],leaf_start,leaf_end)*steps)
        end = math.ceil(prop(starts[i]+lens[i],leaf_start,leaf_end)*steps)
        for j in range(max(start,0),min(end,steps-1)):
            buckets_end[j] = senses[i]
            buckets_num[j] += 1
    # turn into shimmer track
    for i in range(0,steps):
        # calculate blocks
        if buckets_num[i] > 1:
            ops = [(0.0,0.5,not buckets_end[i]),(0.5,1.0,buckets_end[i])]
        elif buckets_num[i] == 1:
            ops = [(0.0,1.0,buckets_end[i])]
        else:
            ops = []
        # set down blocks
        for (start_p,end_p,sense) in ops:
            out_starts.append(leaf_start + (i+start_p)*step_bp)
            out_lens.append((end_p-start_p) * step_bp)
            out_senses.append(sense)
    return (out_starts,out_lens,out_senses)
