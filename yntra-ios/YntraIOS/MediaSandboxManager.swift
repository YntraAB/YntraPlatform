import Foundation
import UIKit
import CryptoKit
import yntra_core

class MediaSandboxManager {
    static let shared = MediaSandboxManager()
    
    private var blobsDirectory: URL {
        let appSupport = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let dir = appSupport.appendingPathComponent("yntra/blobs", isDirectory: true)
        if !FileManager.default.fileExists(atPath: dir.path) {
            try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        }
        return dir
    }
    
    private init() {}
    
    func saveMediaToSandbox(data: Data, mediaType: String = "photo") -> (localURL: URL, hashPointer: String)? {
        let sha256 = SHA256.hash(data: data)
        let hashHex = sha256.compactMap { String(format: "%02x", $0) }.joined()
        let hashPointer = "sha256:\(hashHex)"
        
        let filename = "\(hashHex).dat"
        let fileURL = blobsDirectory.appendingPathComponent(filename)
        
        do {
            try data.write(to: fileURL)
            return (fileURL, hashPointer)
        } catch {
            print("Failed to save media to sandbox: \(error)")
            return nil
        }
    }
    
    func uploadMediaInChunks(
        requesterUserId: String,
        jobId: String,
        mediaData: Data,
        mediaType: String = "photo",
        chunkSize: Int = 512 * 1024,
        completion: @escaping (Result<String, Error>) -> Void
    ) {
        guard let saved = saveMediaToSandbox(data: mediaData, mediaType: mediaType) else {
            completion(.failure(NSError(domain: "YntraMedia", code: 500, userInfo: [NSLocalizedDescriptionKey: "Failed to save sandbox media"])))
            return
        }
        
        let base64String = mediaData.base64EncodedString()
        
        Task {
            do {
                // 1. Enqueue offline media pointer in Rust core
                let pointer = try await enqueueOfflineMediaBlob(
                    requesterUserId: requesterUserId,
                    jobId: jobId,
                    mediaType: mediaType,
                    rawDataBase64: base64String
                )
                
                // 2. Slice into 512KB chunks and upload across UniFFI
                let totalBytes = mediaData.count
                let totalChunks = UInt32(ceil(Double(totalBytes) / Double(chunkSize)))
                
                for chunkIdx in 0..<totalChunks {
                    let start = Int(chunkIdx) * chunkSize
                    let end = min(start + chunkSize, totalBytes)
                    let chunkData = mediaData.subdata(in: start..<end)
                    let chunkBase64 = chunkData.base64EncodedString()
                    
                    _ = try await uploadMediaChunk(
                        requesterUserId: requesterUserId,
                        hashPointer: pointer.hashPointer,
                        chunkIndex: chunkIdx,
                        totalChunks: totalChunks,
                        chunkBase64: chunkBase64
                    )
                }
                
                DispatchQueue.main.async {
                    completion(.success(pointer.hashPointer))
                }
            } catch {
                DispatchQueue.main.async {
                    completion(.failure(error))
                }
            }
        }
    }
}
