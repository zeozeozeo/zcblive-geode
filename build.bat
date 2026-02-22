@echo off
if not exist "build/" (
    mkdir build
    cd build
    cmake .. -GNinja -DCMAKE_BUILD_TYPE=RelWithDebInfo
    cd ..
)

cd live
cargo build --release --features geode
cd ..

cmake --build build --config RelWithDebInfo
if %errorlevel% neq 0 (
    exit /b %errorlevel%
)
