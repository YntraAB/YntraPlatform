import SwiftUI
import UIKit

struct ImagePicker: UIViewControllerRepresentable {
    enum SourceType {
        case camera
        case photoLibrary
    }
    
    var sourceType: SourceType = .camera
    var completion: (UIImage?, URL?) -> Void
    @Environment(\.presentationMode) private var presentationMode
    
    class Coordinator: NSObject, UINavigationControllerDelegate, UIImagePickerControllerDelegate {
        let parent: ImagePicker
        
        init(_ parent: ImagePicker) {
            self.parent = parent
        }
        
        func imagePickerController(_ picker: UIImagePickerController, didFinishPickingMediaWithInfo info: [UIImagePickerController.InfoKey : Any]) {
            if let rawImage = info[.originalImage] as? UIImage {
                // SOTA Memory Optimization: Downsample & compress image on background thread
                DispatchQueue.global(qos: .userInitiated).async {
                    let processed = self.downsampleAndCompress(image: rawImage)
                    DispatchQueue.main.async {
                        self.parent.completion(processed.image, processed.fileURL)
                    }
                }
            } else {
                parent.completion(nil, nil)
            }
            parent.presentationMode.wrappedValue.dismiss()
        }
        
        func imagePickerControllerDidCancel(_ picker: UIImagePickerController) {
            parent.completion(nil, nil)
            parent.presentationMode.wrappedValue.dismiss()
        }
        
        private func downsampleAndCompress(image: UIImage, maxDimension: CGFloat = 1280.0) -> (image: UIImage?, fileURL: URL?) {
            let aspect = image.size.width / image.size.height
            var newSize: CGSize
            if image.size.width > image.size.height {
                newSize = CGSize(width: min(image.size.width, maxDimension), height: min(image.size.width, maxDimension) / aspect)
            } else {
                newSize = CGSize(width: min(image.size.height, maxDimension) * aspect, height: min(image.size.height, maxDimension))
            }
            
            UIGraphicsBeginImageContextWithOptions(newSize, false, 1.0)
            image.draw(in: CGRect(origin: .zero, size: newSize))
            let resizedImage = UIGraphicsGetImageFromCurrentImageContext()
            UIGraphicsEndImageContext()
            
            guard let finalImg = resizedImage, let jpegData = finalImg.jpegData(compressionQuality: 0.8) else {
                return (image, nil)
            }
            
            let tempDir = FileManager.default.temporaryDirectory
            let fileURL = tempDir.appendingPathComponent("yntra_evidence_\(UUID().uuidString).jpg")
            try? jpegData.write(to: fileURL)
            
            return (finalImg, fileURL)
        }
    }
    
    func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }
    
    func makeUIViewController(context: Context) -> UIImagePickerController {
        let picker = UIImagePickerController()
        picker.delegate = context.coordinator
        if sourceType == .camera && UIImagePickerController.isSourceTypeAvailable(.camera) {
            picker.sourceType = .camera
        } else {
            picker.sourceType = .photoLibrary
        }
        return picker
    }
    
    func updateUIViewController(_ uiViewController: UIImagePickerController, context: Context) {}
}
