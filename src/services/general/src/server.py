#! /usr/bin/env python3

from flask import Flask, jsonify
from flask_cors import CORS
import os.path, yaml

import browser.browser

app = Flask(__name__)
CORS(app)

data_path = None
yaml_path = None

app_home_dir = os.path.dirname(os.path.realpath(__file__))
yaml_path = os.path.join(app_home_dir, "yaml")

if 'ett_data_path' in os.environ:
    data_path = os.environ['ett_data_path']
else:
    print ("Data path is not set. Some of the endpoint may not work as expected.")

if 'ett_yaml_path' in os.environ:
    yaml_path = os.environ['ett_yaml_path']
else:
    print ("Using default yaml path")

assets_path = os.path.join(app_home_dir,"assets")
config_path = os.path.join(yaml_path,"config.yaml")
objects_list_path = os.path.join(yaml_path,"example_objects.yaml")
objects_info_path = os.path.join(yaml_path,"objects_info.yaml")

browser_image_bp = browser.browser.browser_setup(config_path,data_path,assets_path)

app.register_blueprint(browser_image_bp)

@app.route("/browser/example_objects")
def example_objects():
    with open(objects_list_path) as f:
        data = yaml.load(f)
        return jsonify(data)


@app.route("/browser/get_object_info/<object_id>")
def get_object_info(object_id):
    with open(objects_info_path) as f:
        data = yaml.load(f)
        if object_id not in data:
          return jsonify({'error':'Object Not Found'})
        else:
          return jsonify(data[object_id])
    
if __name__ == "__main__":
   app.run(port=4000)
