import urllib, urllib.parse, math, random

BLOCK_SIZE = 10000
MAX_BLOCKS = 200

class SequenceCache(object):
    def __init__(self, refget_hashes):
        self.cache = {}
        self.refget_hashes = refget_hashes
        self.hits = 0
        self.misses = 0

    def expand(self,start,end):
        return (math.floor(start/BLOCK_SIZE)*BLOCK_SIZE,
         math.ceil(end/BLOCK_SIZE)*BLOCK_SIZE)

    def decimate(self):
        targets = list(self.cache.keys() or [])
        random.shuffle(targets)
        for key in targets[0:(MAX_BLOCKS//10)]:
            del self.cache[key]

    def get_one(self,hash_,start,end):
        (bstart,bend) = self.expand(start,end)
        k = (hash_,bstart,bend)
        if k in self.cache:
            self.hits += 1
            block = self.cache[k]
        else:
            self.misses += 1
            block = self.refget(*k)
            self.cache[k] = block
        #print("seq cache hit rate",(self.hits*100)/(self.hits+self.misses))
        if len(self.cache) > MAX_BLOCKS:
            self.decimate()
        return block[(start-bstart):(end-bstart)]
        
    def refget(self,hash_,start,end):
        url = ("https://www.ebi.ac.uk/ena/cram/sequence/{}?start={}&end={}"
                .format(hash_,start,end))
        headers = {'Accept': 'text/vnd.ga4gh.refget.v1.0.0+plain;charset=us-ascii'}
        try:
            req = urllib.request.Request(url, None, headers)
            with urllib.request.urlopen(req) as response:
                html = response.read()
                return html.decode("ascii")
        except Exception as e:
            print("url",url)
            return ""
        
    def get(self,chrom,requests):
        #print("get",chrom.seq_hash)
        seq_text = []
        seq_starts = []
        for (start,end) in requests:
            if end > 2:
                if start < 1:
                    start = 1
                seq = self.get_one(chrom.seq_hash,start,end)
                seq_starts.append(start)
                seq_text.append(seq)
        return (seq_text,seq_starts)
