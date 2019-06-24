import math

class Leaf(object):
    def __init__(self,universe,stick,pane):
        self.sticks = universe.get_sticks()
        self._calc_bp_px(pane[0])
        self.leaf_range = self._calc_leaf_range(stick,pane)
        self._burst_leaf()

    def _calc_leaf_range(self,chrom,spec):
        pos = int(spec[1:])
        return "{0}:{1}-{2}".format(chrom,
            math.floor(pos*self.bp_px),
            math.ceil((pos+1)*self.bp_px))

    def _bounds_fix(self,chrom,start,end):
        sticks = self.sticks
        if chrom in sticks:
            f_len = sticks[chrom]
            f_len = int(f_len)
            if end >= f_len:
                end = int(f_len)-1
            if start >= f_len:
                start = int(f_len)-2                    
        if start < 0:
            start = 0
        return (start,end)

    def _burst_leaf(self):
        (self.chrom,rest) = self.leaf_range.rsplit(':',1)
        (start,end) = rest.split('-',1)
        try:
            (start,end) = (int(start),int(end))
        except ValueError:
            (start,end) = (1,2)
        (self.start,self.end) = self._bounds_fix(self.chrom,start,end)

    def _calc_bp_px(self,spec):
        spec_number = ord(spec) - ord('A') - 13
        bp_px = 10**(math.floor(abs(spec_number)/2))
        if abs(spec_number) % 2:
            bp_px *= 3
        if spec_number > 0:
            bp_px = 1.0 / bp_px
        self.bp_px = bp_px * 5000
