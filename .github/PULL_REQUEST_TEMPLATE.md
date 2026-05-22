---
name: Pull request
about: Mô tả thay đổi pull request
---

## Mô tả thay đổi
Tóm tắt ngắn phạm vi thay đổi. Cố gắng chỉ ra big picture của thay đổi.

## Lý do thay đổi
Tại sao thay đổi này cần thiết? Lợi ích vs rủi ro?

## Thay đổi cụ thể
- [ ] Done

## Bản tóm tắt
- Phong cách code: đặc biệt chú ý bất kỳ sự thay đổi nào liên quan đến an toàn bộ nhớ, đa luồng
- API: đã có breaking change? đã bổ sung test?
- Tính tương thích: vẫn tương thích ngược? Tránh những thay đổi phá vỡ cho người dùng hiện tại
- Migration: đã cung cấp hướng dẫn di chuyển nếu có breaking change?

## Bản demo
Dán screenshot/video nếu thay đổi là UI/UX. Nếu là performance, bao gồm benchmarks qua `--release` mode.

## Kiểm tra trước khi merge
- [ ] Test case bao trùm hết logic code mới
- [ ] CI xanh (chạy `cargo test`)
- [ ] Build release không lỗi
- [ ] Không warnings trên CI (chạy `cargo clippy`)