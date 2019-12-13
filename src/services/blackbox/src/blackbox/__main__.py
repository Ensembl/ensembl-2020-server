from flask import Flask
from flask_cors import CORS
from server import blackbox

if __name__ == "__main__":
    app = Flask(__name__)
    app.register_blueprint(blackbox(),url_prefix="/blackbox")
    CORS(app)
    app.run(port=4040,host='0.0.0.0')
