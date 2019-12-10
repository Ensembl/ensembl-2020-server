import os
import unittest

from server import create_app

EXAMPLE_DATA = """
[
    {"instance":"test1","stack":["a","b"],"text":"Hello, world!","time":2.0},
    {"data":[2.0],"dataset":"raw","instance":"test1","text":"raw elapsed: num=1 total=2.00units avg=2.00units 95%ile=2.00units top=2.00units","time":2.0}
]
"""

class BasicTests(unittest.TestCase): 
    def setUp(self):
        self.app = create_app(testing=True).test_client()
 
    def tearDown(self):
        pass

    def test_config(self):
        response = self.app.post('/blackbox/update-config',data = {
            "enable": "test1"
        })
        self.assertEqual(response.status_code,200)

    def test_smoke(self):
        response = self.app.post('/blackbox/data',data=EXAMPLE_DATA)
        self.assertEqual(response.status_code,200)
 
 
if __name__ == "__main__":
    unittest.main()