import SwiftUI

struct DynamicFieldSchema: Identifiable {
    let id = UUID()
    let key: String
    let label: String
    let type: String
    let required: Bool
}

struct DynamicBlockView: View {
    let blockId: String
    let blockName: String
    let schemaJson: String
    let dataJson: String
    let onSaveData: (String) -> Void

    @State private var fieldValues: [String: String] = [:]
    @State private var schemaFields: [DynamicFieldSchema] = []

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Text(blockName)
                    .font(.system(size: 26, weight: .black))
                    .foregroundColor(.white)

                Spacer()

                Button(action: {
                    if let jsonData = try? JSONSerialization.data(withJSONObject: fieldValues, options: []),
                       let jsonString = String(data: jsonData, encoding: .utf8) {
                        onSaveData(jsonString)
                    }
                }) {
                    HStack(spacing: 6) {
                        Image(systemName: "checkmark")
                        Text("Save Block")
                            .fontWeight(.bold)
                    }
                    .padding(.horizontal, 14)
                    .padding(.vertical, 8)
                    .background(Color(red: 0.31, green: 0.27, blue: 0.90))
                    .foregroundColor(.white)
                    .cornerRadius(10)
                }
            }

            if schemaFields.isEmpty {
                VStack(spacing: 12) {
                    Spacer()
                    Image(systemName: "cpu")
                        .font(.system(size: 40))
                        .foregroundColor(Color(red: 0.58, green: 0.64, blue: 0.72))

                    Text("Dynamic Operational Block (\(blockId))")
                        .font(.headline)
                        .foregroundColor(.white)

                    Text("Schema active and ready for field data inputs.")
                        .font(.subheadline)
                        .foregroundColor(Color(red: 0.58, green: 0.64, blue: 0.72))
                    Spacer()
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(Color(red: 0.12, green: 0.16, blue: 0.23))
                .cornerRadius(16)
            } else {
                ScrollView {
                    VStack(spacing: 14) {
                        ForEach(schemaFields) { field in
                            VStack(alignment: .leading, spacing: 8) {
                                Text("\(field.label)\(field.required ? " *" : "")")
                                    .font(.subheadline)
                                    .fontWeight(.semibold)
                                    .foregroundColor(.white)

                                TextField("Enter \(field.label)", text: Binding(
                                    get: { fieldValues[field.key] ?? "" },
                                    set: { fieldValues[field.key] = $0 }
                                ))
                                .padding(12)
                                .background(Color(red: 0.06, green: 0.09, blue: 0.16))
                                .foregroundColor(.white)
                                .cornerRadius(10)
                                .overlay(
                                    RoundedRectangle(cornerRadius: 10)
                                        .stroke(Color(red: 0.20, green: 0.25, blue: 0.33), lineWidth: 1)
                                )
                            }
                            .padding(16)
                            .background(Color(red: 0.12, green: 0.16, blue: 0.23))
                            .cornerRadius(14)
                        }
                    }
                }
            }
        }
        .padding(20)
        .background(Color(red: 0.04, green: 0.06, blue: 0.10).edgesIgnoringSafeArea(.all))
        .onAppear {
            parseSchemaAndData()
        }
    }

    private func parseSchemaAndData() {
        if let data = dataJson.data(using: .utf8),
           let dict = try? JSONSerialization.jsonObject(with: data, options: []) as? [String: String] {
            self.fieldValues = dict
        }

        if let data = schemaJson.data(using: .utf8),
           let array = try? JSONSerialization.jsonObject(with: data, options: []) as? [[String: Any]] {
            self.schemaFields = array.compactMap { obj in
                guard let key = obj["key"] as? String,
                      let label = obj["label"] as? String else { return nil }
                return DynamicFieldSchema(
                    key: key,
                    label: label,
                    type: obj["type"] as? String ?? "text",
                    required: obj["required"] as? Bool ?? false
                )
            }
        }
    }
}
