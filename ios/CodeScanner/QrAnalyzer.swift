import Accelerate
import AVFoundation
import CoreImage
import Foundation
import UIKit
import Vision

class QrAnalyzer: NSObject, AVCaptureVideoDataOutputSampleBufferDelegate {
    private var lastAnalysisTime: Date = .distantPast
    private let analysisCooldown: TimeInterval = 0.2 // 400ms

    public func captureOutput(
        _: AVCaptureOutput, didOutput sampleBuffer: CMSampleBuffer, from _: AVCaptureConnection
    ) {
        let currentTime = Date()
        guard currentTime.timeIntervalSince(lastAnalysisTime) >= analysisCooldown else {
            return
        }

        guard let pixelBuffer = CMSampleBufferGetImageBuffer(sampleBuffer) else {
            return
        }

        let ciImage = CIImage(cvPixelBuffer: pixelBuffer)
        let context = CIContext()
        guard let cgImage = context.createCGImage(ciImage, from: ciImage.extent) else {
            return
        }

        let image = UIImage(cgImage: cgImage)

        Task {
            self.analyzeImage(image)
        }
    }

    private func analyzeImage(_ image: UIImage) {
        Task {
            do {
                let potentialQRCodes = try await detectPotentialQRCodes(in: image)
                try await detectPotentialQRCodes2(in: image)
                if !potentialQRCodes.isEmpty {
//                    checkImageBlurriness(in: image, potentialQRCodes: potentialQRCodes)
//                    checkPotentialQrCodeSizes(rectangles: potentialQRCodes, in: image)
                }
            } catch {
                print("Failed to detect potential QR codes: \(error)")
            }
        }
    }

    private func detectPotentialQRCodes2(in image: UIImage) async throws -> [VNDetectedObjectObservation] {
        guard let ciImage = CIImage(image: image) else { throw NSError(domain: "ImageConversion", code: 0, userInfo: nil) }

        let request = VNDetectRectanglesRequest()
        request.minimumAspectRatio = 0.85
        request.maximumAspectRatio = 1.15
        request.quadratureTolerance = 0.1
        request.minimumSize = 0.1
        request.maximumObservations = 0 // No limit

        let handler = VNImageRequestHandler(ciImage: ciImage, orientation: .up, options: [:])
        try handler.perform([request])

        guard let results = request.results else { return [] }

        return results.filter { observation in
            let confidence = observation.confidence
            print("Detected shape confidence: \(confidence)")
            return confidence > 0 // Adjust this threshold as needed
        }
    }

    private func detectPotentialQRCodes(in image: UIImage) async throws -> [VNBarcodeObservation] {
        guard let cgImage = image.cgImage else { return [] }

        let request = VNDetectBarcodesRequest()
        let handler = VNImageRequestHandler(cgImage: cgImage, orientation: .up)

        try handler.perform([request])

        guard let results = request.results else { return [] }

        return results.filter { result in
            print("Symbology: \(result.symbology), Confidence: \(result.confidence), Payload: \(result.payloadStringValue ?? "nil")")
            return result.symbology == .qr && result.confidence > 0 && result.payloadStringValue == nil
        }
    }

    private func checkImageBlurriness(in image: UIImage, potentialQRCodes: [VNRectangleObservation]) {
        guard let cgImage = image.cgImage else { return }

        for rectangle in potentialQRCodes {
            let boundingBox = rectangle.boundingBox

            let ciImage = CIImage(cgImage: cgImage)
            let croppedImage = ciImage.cropped(to: boundingBox)

            if let contours = detectContours(of: croppedImage) {
                print("CONTOURS", contours)
            } else {
                print("coudl not  detect contours")
            }

//            if blurriness < 10 { // Adjust this threshold as needed
//                print("Potential QR code is too blurry. Please hold the camera steady.")
//            } else {
//                print("Potential QR code detected with acceptable sharpness.")
//            }
        }
    }

    private func checkPotentialQrCodeSizes(rectangles: [VNBarcodeObservation], in _: UIImage) {
        for rectangle in rectangles {
            let boundingBox = rectangle.boundingBox
            let size = boundingBox.size

            print("Potential QR code size: \(size)")

            // Check if the potential QR code is too small
            if size.width < 0.1 || size.height < 0.1 {
                print("Potential QR code is too small. Please move closer.")
            } else {
                // You can add more analysis here, such as checking for typical QR code patterns
                print("Potential QR code detected at \(boundingBox)")
            }
        }
    }

    private func detectContours(of inputImage: CIImage) -> Float? {
        let request = VNDetectContoursRequest()
        request.contrastAdjustment = 2.0 // Enhance contrast for better edge detection
        request.maximumImageDimension = 500 // Adjust based on your needs

        let handler = VNImageRequestHandler(ciImage: inputImage)
        try? handler.perform([request])

        if let contoursObservation = request.results?.first as? VNContoursObservation {
            let totalContourLength = getTotalContourLength(from: contoursObservation)

            // Normalize by image size
            let imageSize = inputImage.extent.size
            let normalizedLength = totalContourLength / (Double(imageSize.width) * Double(imageSize.height))
            return Float(normalizedLength)
        }

        return nil
    }

    func getTotalContourLength(from observation: VNContoursObservation) -> Double {
        var totalLength = 0.0

        for i in 0..<observation.contourCount {
            guard let contour = try? observation.contour(at: i) else { continue }
            totalLength += contourLength(contour)
        }

        return totalLength
    }

    func contourLength(_ contour: VNContour) -> Double {
        let points = contour.normalizedPoints
        var length = 0.0

        for i in 1..<points.count {
            let p1 = points[i - 1]
            let p2 = points[i]
            length += hypot(Double(p2.x - p1.x), Double(p2.y - p1.y))
        }

        // Close the contour if it's not already closed
        if points.first != points.last {
            let p1 = points.last!
            let p2 = points.first!
            length += hypot(Double(p2.x - p1.x), Double(p2.y - p1.y))
        }

        return length
    }
}
