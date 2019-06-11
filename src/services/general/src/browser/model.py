import math, yaml

class Leaf(object):
    def __init__(self,sticks,stick,pane):
        self.sticks = sticks
        self._calc_bp_px(pane[0])
        self.leaf_range = self._calc_leaf_range(stick,pane)
        self._burst_leaf()

    def _calc_leaf_range(self,chrom,spec):
        pos = int(spec[1:])
        return "{0}:{1}-{2}".format(chrom,
            math.floor(pos*self.bp_px),
            math.ceil((pos+1)*self.bp_px))

    def _bounds_fix(self,chrom,start,end):
        sticks = self.sticks.get_sticks()
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


class Sticks(object):
    def __init__(self,chrom_sizes):
        self.chrom_sizes = chrom_sizes
    
    def get_sticks(self):
        out = {}
        with open(self.chrom_sizes) as f:
            for line in f.readlines():
                (f_chr,f_len) = line.strip().split("\t")
                out[f_chr] = f_len
        out["text2"] = "1000000"
        return out

class BAIConfig(object):
    def __init__(self,config_path,assets_path):
        self.config_path = config_path
        self.assets_path = assets_path
        self.endpoints = {}
        self.bytecodes = {}
        self.tracks = {}
        self._load()
        
    def _load(self):
        ep_map = {}
        bc_map = {}
        with open(self.config_path) as f:
            bc = yaml.load(f)
            for (ep_name,v) in bc["endpoints"].items():
                if "endpoint" in v:
                    ep_map[ep_name] = v["endpoint"]
                if "bytecode" in v:
                    bc_map[ep_name] = v["bytecode"]
            for (track_name,v) in bc["tracks"].items():
                for (code,v) in v["endpoints"].items():
                    for scale in range(ord(code[0]),ord(code[1])+1):
                        if v["endpoint"] in ep_map:
                            self.endpoints[(track_name,chr(scale))] = ep_map[v["endpoint"]]
                        if v["endpoint"] in bc_map:
                            self.bytecodes[(track_name,chr(scale))] = bc_map[v["endpoint"]]
            for (t_name,v) in bc["tracks"].items():
                if "wire" in v:
                    self.tracks[v["wire"]] = t_name
