fn main() {
    // Đảm bảo APP_SECRET được set khi build (để embed vào binary qua env!())
    // Nếu không set, dùng giá trị dev mặc định
    if std::env::var("APP_SECRET").is_err() {
        println!("cargo:rustc-env=APP_SECRET=dev-secret-replace-in-production");
    }
    tauri_build::build()
}
