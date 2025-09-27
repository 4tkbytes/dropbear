import Foundation
#if canImport(Network)
import Network
#endif

/// Cross-platform TCP socket client for communicating with the Rust engine
@available(macOS 10.15, iOS 13.0, watchOS 6.0, tvOS 13.0, *)
public class SocketClient: @unchecked Sendable {
    private let host: String
    private let port: Int
    
    #if canImport(Network)
    private var connection: NWConnection?
    #endif
    
    public init(host: String = "127.0.0.1", port: Int = 7878) {
        self.host = host
        self.port = port
    }
    
    /// Connect to the socket server
    public func connect() async throws {
        #if canImport(Network)
        let nwEndpoint = NWEndpoint.hostPort(host: NWEndpoint.Host(host), port: NWEndpoint.Port(integerLiteral: UInt16(port)))
        let connection = NWConnection(to: nwEndpoint, using: .tcp)
        
        return try await withCheckedThrowingContinuation { continuation in
            connection.stateUpdateHandler = { state in
                switch state {
                case .ready:
                    continuation.resume()
                case .failed(let error):
                    continuation.resume(throwing: SocketError.connectionFailed(error.localizedDescription))
                case .cancelled:
                    continuation.resume(throwing: SocketError.connectionFailed("Connection cancelled"))
                default:
                    break
                }
            }
            
            connection.start(queue: .global())
            self.connection = connection
        }
        #else
        throw SocketError.connectionFailed("Network framework not available on this platform")
        #endif
    }
    
    /// Disconnect from the socket server
    public func disconnect() async {
        #if canImport(Network)
        connection?.cancel()
        connection = nil
        #endif
    }
    
    /// Send a request and receive a response
    public func sendRequest<T: Codable>(_ request: EngineRequest) async throws -> T {
        #if canImport(Network)
        guard let connection = self.connection else {
            throw SocketError.notConnected
        }
        
        // Serialize request to JSON
        let requestData = try JSONEncoder().encode(request)
        let requestString = String(data: requestData, encoding: .utf8)! + "\n"
        let requestBytes = Data(requestString.utf8)
        
        // Send request
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            connection.send(content: requestBytes, completion: .contentProcessed { error in
                if let error = error {
                    continuation.resume(throwing: SocketError.connectionFailed(error.localizedDescription))
                } else {
                    continuation.resume()
                }
            })
        }
        
        // Receive response
        let responseData = try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Data, Error>) in
            connection.receive(minimumIncompleteLength: 1, maximumLength: 4096) { data, _, isComplete, error in
                if let error = error {
                    continuation.resume(throwing: SocketError.connectionFailed(error.localizedDescription))
                } else if let data = data {
                    continuation.resume(returning: data)
                } else if isComplete {
                    continuation.resume(throwing: SocketError.connectionFailed("Connection closed"))
                }
            }
        }
        
        // Parse response
        let responseString = String(data: responseData, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        guard let finalResponseData = responseString.data(using: .utf8) else {
            throw SocketError.decodingError
        }
        
        return try JSONDecoder().decode(T.self, from: finalResponseData)
        
        #else
        throw SocketError.connectionFailed("Network framework not available on this platform")
        #endif
    }
    
    /// Send an engine request with automatic response type handling
    public func sendEngineRequest(_ request: EngineRequest) async throws -> EngineResponse {
        return try await sendRequest(request)
    }
}

/// Socket-related errors
public enum SocketError: Error, LocalizedError {
    case notConnected
    case connectionFailed(String)
    case invalidResponse
    case encodingError
    case decodingError
    
    public var errorDescription: String? {
        switch self {
        case .notConnected:
            return "Socket is not connected"
        case .connectionFailed(let reason):
            return "Connection failed: \(reason)"
        case .invalidResponse:
            return "Invalid response from server"
        case .encodingError:
            return "Failed to encode request"
        case .decodingError:
            return "Failed to decode response"
        }
    }
}