# 🚀 GeoSign

## 🚧 Getting Started

Ready to dive in? Follow these simple steps to get your GeoSign up and running!

### Prerequisites

Make sure you have the following installed:

- **Rust 1.8+**
- **Docker 24.0+**

### Installation

Clone the repository and build the project:

```bash
git clone https://github.com/fadimanakilci/geosign.git
cd geosign
```

### Download and Run Qdrant

Download the latest Qdrant image from Dockerhub:

```bash
docker pull qdrant/qdrant
```

Then, run the service:

```bash
docker run -p 6333:6333 -p 6334:6334 \
    -v $(pwd)/qdrant_storage:/qdrant/storage:z \
    qdrant/qdrant
```

And you’re good to go! 🚀
<br><br>

## 🤝 Contributing
We’d love your help in making Custom Message Broker even better! Here’s how you can contribute:

1. Fork the repository to your own GitHub account.
2. Create a branch for your feature: git checkout -b feature/AmazingFeature.
3. Commit your changes: git commit -m 'Add some AmazingFeature'.
4. Push to your branch: git push origin feature/AmazingFeature.
5. Open a Pull Request and let’s make this broker even more awesome together!
<br><br>

## 📄 License
This project is licensed under the MIT License – see the [LICENSE](./LICENSE) file for details.