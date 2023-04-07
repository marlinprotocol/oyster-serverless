import json
from locust import HttpUser, task, between

class WebsiteUser(HttpUser):
    wait_time = between(1, 5)

    @task
    def index(self):
        data = {
            "tx_hash": "0xc7d9122f583971d4801747ab24cf3e83984274b8d565349ed53a73e0a547d113",
            "input": {
                "num": 100
            }
        }
        headers = {'Content-Type': 'application/json'}

        self.client.post("/api/serverless", data=json.dumps(data), headers=headers)
