import Foundation
import Combine
import MyFramework

// 네트워크 요청을 관리하는 클래스
final class NetworkManager {

    // 공유 인스턴스 (싱글턴)
    static let shared = NetworkManager()

    private init() {}

    /// 지정된 URL에서 데이터를 가져옵니다.
    /// - Parameter url: 요청할 URL
    /// - Returns: 데이터 퍼블리셔
    func fetch(from url: URL) -> AnyPublisher<Data, Error> {
        /* URLSession 데이터 태스크:
           네트워크 요청을 Combine 퍼블리셔로 래핑합니다.
        */
        return URLSession.shared.dataTaskPublisher(for: url)
            .map(\.data)
            .mapError { $0 as Error }
            .eraseToAnyPublisher()
    }

    // 기본 헤더 설정
    private func defaultHeaders() -> [String: String] {
        let version = "애플리케이션 버전 1.0" // 버전 문자열
        return ["X-App-Version": version]
    }
}
