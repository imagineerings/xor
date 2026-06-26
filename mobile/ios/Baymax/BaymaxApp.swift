import SwiftUI

@main
struct BaymaxApp: App {
    @StateObject private var configurationHandler = ConfigurationHandler.shared
    @StateObject private var noticeCenter = AppNoticeCenter.shared
    
    init() {
        // Set demo defaults on first launch only
        initializeDefaultsIfNeeded()
    }
    
    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(configurationHandler)
                .environmentObject(noticeCenter)
                .onOpenURL { url in
                    print("📱 App received URL: \(url)")
                    _ = configurationHandler.handleURL(url)
                }
        }
    }
    
    /// Initialize demo defaults only if user has never set configuration
    private func initializeDefaultsIfNeeded() {
        let hasConfiguredURL = UserDefaults.standard.object(forKey: "baymax_base_url") != nil
        let hasConfiguredSecret = UserDefaults.standard.object(forKey: "baymax_secret_key") != nil
        
        // Only set defaults if neither URL nor secret has been configured
        if !hasConfiguredURL && !hasConfiguredSecret {
            print("🎯 First launch detected - setting demo defaults")
            UserDefaults.standard.set("https://demo-baymaxd.fly.dev", forKey: "baymax_base_url")
            UserDefaults.standard.set("test", forKey: "baymax_secret_key")
            UserDefaults.standard.synchronize()
        }
    }
}
