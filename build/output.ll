; ModuleID = 'main_module'
target triple = "aarch64-apple-darwin"

declare i32 @puts(i8*)
declare i8* @malloc(i64)
declare void @free(i8*)

define i64 @native_add() {
bb_4:
  %r9 = add i64 %r7, %r8
  ret i64 %r9
}

define void @main() {
bb_0:
  %r0 = alloca i64
  %r1 = call i64 @native_add(i64 40, i64 2)
  store i64 %r1, i64* %r0
  %r2 = load i64, i64* %r0
  %r3 = add i64 %r2, 42
  %r4 = alloca i64
  br i1 %r3, label %bb_1, label %bb_2
bb_1:
  %r5 = call i64 @println(i64 0)
  store i64 0, i64* %r4
  br label %bb_3
bb_2:
  store i64 0, i64* %r4
  br label %bb_3
bb_3:
  %r6 = load i64, i64* %r4
  ret void
}

