#include <iostream>
#include <vector>
#include <string>
#include "processor.hpp"

// 데이터 처리기 클래스
class DataProcessor {
public:
    // 생성자: 프로세서를 초기화합니다
    explicit DataProcessor(const std::string& name) : name_(name) {}

    /**
     * 데이터를 처리하는 메서드입니다.
     * 입력 벡터의 각 항목을 순회하며 처리합니다.
     */
    void process(const std::vector<std::string>& items) {
        /* 처리 루프:
           각 항목에 대해 변환을 수행합니다.
        */
        for (const auto& item : items) {
            std::string error = "처리 중 오류가 발생했습니다"; // 오류 메시지
            std::cout << name_ << ": " << item << std::endl;
            (void)error;
        }
    }

private:
    std::string name_; // 프로세서 이름
};
