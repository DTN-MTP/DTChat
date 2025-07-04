# DTChat - Delay Tolerant Network Chat Application

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE) [![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

DTChat is a modern, GUI-based chat application designed for Delay Tolerant Networks (DTN) using Bundle Protocol communication. Built with Rust and egui, it provides real-time messaging with predictive delivery times through advanced routing algorithms.

## Features

- **Bundle Protocol Integration**: Native support for ION-DTN and bp-socket
- **PBAT (Predicted Bundle Arrival Time)**: Real-time delivery predictions using A-SABR routing algorithms
- **Modern GUI**: Clean, responsive interface built with egui framework
- **Multiple Views**: Message list, timeline graph, and data table visualizations
- **Protobuf Support**: Efficient message serialization and ACK support
- **Real-time Updates**: Live message status and delivery confirmations
- **Contact Plan Integration**: ION contact plan parsing for optimal routing

## Architecture

```
DTChat/
├── src/
│   ├── app.rs                      # Main application logic
│   ├── main.rs                     # Entry point
│   ├── layout/                     # UI components
│   │   ├── rooms/                  # Chat interface modules
│   │   │   ├── message_list.rs     # Message display
│   │   │   ├── message_prompt.rs   # Input handling
│   │   │   ├── message_forge.rs    # Peer selection
│   │   │   └── message_graph.rs    # Timeline visualization
│   │   └── menu_bar.rs             # Navigation menu
│   ├── utils/                      # Core utilities
│   │   ├── prediction_config.rs    # Prediction implementation using A-SABR routing
│   │   ├── proto.rs                # Message serialization
│   │   ├── socket.rs               # Network communication
│   │   ├── message.rs              # Message data structures
│   │   ├── config.rs               # Configuration management
│   │   └── ack.rs                  # Acknowledgment handling
│   └── proto/                      # Protocol buffer definitions
│       └── message.proto           # Message format specification
├── A-SABR/                         # Routing algorithm submodule
├── database.yaml                   # Peer and network configuration
└── Cargo.toml                      # Dependencies and features
```

## Quick Start

### Prerequisites

- **Rust 1.70+**: [Install Rust](https://rustup.rs/)
- **ION-DTN or bp-socket**: Bundle Protocol daemon
- **Git**: For submodule support
- [Protobuf](https://protobuf.dev/installation/)

### Installation

```bash
# Clone the repository with submodules
git clone --recursive https://github.com/DTN-MTP/DTChat.git
cd DTChat

# in the node VM Build the project
cargo build --release

# Run DTChat with default configuration
DTCHAT_CONFIG=./db/default.yaml  cargo run 
```

### Configuration (DTCHAT_CONFIG)

Three different configuration files are available in the `db` directory:
- `default.yaml`: Default configuration for local testing
- `ion.yaml`: Example configuration for ion integration (dtchat-bp-socket-testing)
- `ud3dtn.yaml`: Example configuration for ud3dtn integration(dtchat-bp-socket-testing)


**Configure contact plan (example)**:

```
# ION Administration
a contact +0 +86400 30 30 100000
a range +0 +86400 30 30 1

# Bundle Protocol 
a protocol tcp 1400 100
a induct tcp 192.168.50.10:4556 tcpcli
a outduct tcp 192.168.50.30:4556 tcpclo
```

## Usage

### Basic Chat

1. **Start DTChat**: `DTCHAT_CONFIG=./db/default.yaml cargo run`
2. **Select a peer** from the dropdown menu
3. **Click on the PBAT checkbox (optional)** to view delivery time prediction
4. **Type your message** in the input field
5. **Press Enter or click Send**
6. **View delivery predictions** in real-time (if PBAT enabled)

![DTChat Main Interface ](docs/img/DTChat%20Graph%20view%20with%20pbat.png)
*DTChat Main Interface Showing Type Of Messages (Sent, Sent with PBAT enabled and Received Messages)*

### Message Status Indicators

- **Sent Messages**: `[sent_time->predicted_time][sender]`
- **Received Messages**: `[sent_time->received_time✓][sender]`
- **Failed Delivery**: Error indicators and retry options


### Views

- **List View**: Chronological message display
- **Graph View**: Timeline with delivery predictions
- **Table View**: Structured data with timestamps

## Development

### Building Features

```bash
# Development build with debug features
cargo run --features dev


### Project Structure

- **UI Components**: `src/layout/` - egui-based interface modules
- **Network Layer**: `src/utils/socket.rs` - Bundle Protocol communication
- **PBAT Using Routing Algorithms**: `src/utils/prediction_config.rs` - A-SABR integration
- **Message Handling**: `src/utils/message.rs` - Data structures and serialization
- **Configuration**: `src/utils/config.rs` - YAML-based configuration


## Advanced Features

### A-SABR Routing Integration

DTChat integrates with the A-SABR (Adaptive Schedule-Aware Bundle Routing) framework for:
- **Contact Plan Analysis**: Parses ION contact plans for network topology
- **Route Optimization**: Calculates optimal paths based on contact schedules
- **Delivery Prediction**: Estimates message arrival times with high accuracy
- **Dynamic Adaptation**: Adjusts routes based on network conditions

### Protocol Buffer Support

Efficient message serialization with:
- **Message Types**: Text, ACK, status updates
- **Delivery Tracking**: Message UUIDs and delivery confirmations
- **Compression**: Optimized for bandwidth-constrained networks
- **Compatibility**: Backward compatibility with text-mode debugging

### Network Protocols

Supports multiple transport mechanisms:
- **Bundle Protocol**: Native DTN communication
- **TCP/UDP**: Traditional networking for testing
- **ION Integration**: Direct integration with NASA's ION-DTN
- **bp-socket**: Kernel-level Bundle Protocol support

## Troubleshooting

### Common Issues

**"No route found"**
```bash
# Check contact plan configuration
cat <contact_plan file>

# Verify ION daemon status  
ionadmin

# Check database.yaml configuration
cat database.yaml
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- **A-SABR Framework**: Advanced routing algorithms for DTN networks
- **ION-DTN**: NASA's Interplanetary Overlay Network
- **egui**: Immediate mode GUI framework for Rust
- **DTN Research Community**: For delay-tolerant networking innovations

## Related Projects

- **[A-SABR](https://github.com/DTN-MTP/A-SABR)**: Adaptive Schedule-Aware Bundle Routing
- **[bp-socket](https://github.com/DTN-MTP/bp-socket)**: Kernel-level Bundle Protocol support
- **[ION-DTN](https://sourceforge.net/projects/ion-dtn/)**: NASA's DTN implementation

---
