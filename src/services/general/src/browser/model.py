import math, yaml, re, os.path

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

class Chromosome(object):
    def __init__(self,universe,species,parts,hash_):
        self.config_path = universe.config_path
        self.name = parts[0]
        self.size = int(parts[1])
        self.seq_hash = hash_
        self.stick_name = "{0}:{1}".format(
            species.wire_genome_id,self.name
        )
        self.genome_path = species.genome_id
        universe._add_chrom(self)

    def file_path(self,section,filename):
        return os.path.join(self.config_path,section,self.genome_path,filename)

class Species(object):
    def __init__(self,universe,name,config):
        self.production_name = name
        self.genome_id = config['genome_id']
        self.gca = config['gca']
        self.species = config['species']
        self.dbname = config['dbname']
        self.version = config['version']
        self.wire_genome_id = re.sub(r'\W','_',self.genome_id)
        self.genome_path = self.genome_id
        self.config_path = universe.config_path
        self.chroms = {}
        self._load(universe)
        
    def _load(self,universe):
        hashes = {}
        with open(self.file_path("chrom.hashes")) as f:
            for line in f.readlines():
                parts = line.strip().split("\t")
                hashes[parts[0]] = parts[1]        
        with open(self.file_path("chrom.sizes")) as f:
            for line in f.readlines():
                parts = line.strip().split("\t")
                hash_ = hashes[parts[0]]
                chrom = Chromosome(universe,self,parts,hash_)
                self.chroms[chrom.name] = chrom

    def file_path(self,filename):
        return os.path.join(self.config_path,"common_files",self.genome_path,filename)


class Universe(object):
    def __init__(self,config_path):
        self.config_path = config_path
        self.species = {}
        self.sticks = {}
        self._load()
        
    def _load(self):
        with open(os.path.join(self.config_path,"common_files","genome_id_info.yml")) as f:
            config = yaml.load(f)
            for (k,v) in config.items():
                s = Species(self,k,v)
                self.species[s.wire_genome_id] = s
                
    def _add_chrom(self,chrom):
        self.sticks[chrom.stick_name] = chrom

    def get_sticks(self):
        return { k: str(v.size) for (k,v) in self.sticks.items() }
        
    def get_from_stick(self,stick):
        return self.sticks[stick]

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
