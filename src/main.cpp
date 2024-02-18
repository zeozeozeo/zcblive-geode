#include "Geode/GeneratedPredeclare.hpp"
#include <Geode/Geode.hpp>
#include <Geode/modify/MenuLayer.hpp>
#include <Geode/modify/PlayLayer.hpp>
#include <Geode/modify/GJBaseGameLayer.hpp>
#include <fstream>
#include "embed.hpp"
#include <mutex>

// windows.h
#define WIN32_LEAN_AND_MEAN
#include <windows.h>

using namespace geode::prelude;

// zcblive_on_action
typedef void (*zcblive_on_action_fn)(uint8_t button, bool player2);

// zcblive_set_playlayer
// typedef void (*zcblive_set_playlayer_fn)(void* playlayer);

// zcblive_set_is_in_level
typedef void (*zcblive_set_is_in_level_fn)(bool is_in_level);

// zcblive_set_playlayer_time
typedef void (*zcblive_set_playlayer_time_fn)(double playlayer_time);

// zcblive_on_playlayer_init
typedef void (*zcblive_on_playlayer_init_fn)();

// zcblive_on_basegamelayer_reset
typedef void (*zcblive_on_basegamelayer_reset_fn)();

zcblive_on_action_fn zcblive_on_action = nullptr;
// zcblive_set_playlayer_fn zcblive_set_playlayer = nullptr;
zcblive_set_is_in_level_fn zcblive_set_is_in_level = nullptr;
zcblive_set_playlayer_time_fn zcblive_set_playlayer_time = nullptr;
zcblive_on_playlayer_init_fn zcblive_on_playlayer_init = nullptr;
zcblive_on_basegamelayer_reset_fn zcblive_on_basegamelayer_reset = nullptr;

std::mutex g_mutex;

$on_mod(Loaded) {
	Mod::get()->setLoggingEnabled(true);

	// write zcblive.dll
	{
		std::ofstream f;
		f.open("zcblive.dll", std::ios::binary | std::ios::out | std::ios::trunc);
		if (!f.is_open()) {
			log::error("failed to open zcblive.dll");
			return;
		}
		f.write(reinterpret_cast<const char*>(zcblive_dll.data()), zcblive_dll.size());
		if (!f) {
			log::error("error writing to zcblive.dll");
			return;
		}
		f.close();
	}

	// inject it into the game
	const char* dll_path = "zcblive.dll";
	HMODULE dll_handle = LoadLibraryA(dll_path);
	if (!dll_handle) {
		DWORD error_code = GetLastError();
		switch (error_code) {
		case ERROR_FILE_NOT_FOUND:
			log::error("DLL not found at specified path: {}", dll_path);
			break;
		case ERROR_MOD_NOT_FOUND:
			log::error("DLL format invalid or corrupt: {}", dll_path);
			break;
		case ERROR_ACCESS_DENIED:
			log::error("Unable to access DLL due to permissions: {}", dll_path);
			break;
		default:
			log::error("Unexpected error loading DLL (code: {}). path: {}", error_code, dll_path);
		}
		return;
	}

	// load functions (IT WORKS OK)
	zcblive_on_action = (zcblive_on_action_fn)GetProcAddress(dll_handle, "zcblive_on_action");
	// zcblive_set_playlayer = (zcblive_set_playlayer_fn)GetProcAddress(dll_handle, "zcblive_set_playlayer");
	zcblive_set_is_in_level = (zcblive_set_is_in_level_fn)GetProcAddress(dll_handle, "zcblive_set_is_in_level");
	zcblive_set_playlayer_time = (zcblive_set_playlayer_time_fn)GetProcAddress(dll_handle, "zcblive_set_playlayer_time");
	zcblive_on_playlayer_init = (zcblive_on_playlayer_init_fn)GetProcAddress(dll_handle, "zcblive_on_playlayer_init");
	zcblive_on_basegamelayer_reset = (zcblive_on_basegamelayer_reset_fn)GetProcAddress(dll_handle, "zcblive_on_basegamelayer_reset");
	if (!zcblive_on_action || !zcblive_set_is_in_level ||
		!zcblive_set_playlayer_time || !zcblive_on_playlayer_init || !zcblive_on_basegamelayer_reset) {
		log::error("one or more functions not found in DLL.");
		FreeLibrary(dll_handle);
	}
}

enum {
	PB_PUSH,
	PB_RELEASE,
	PB_LEFT,
	PB_RIGHT,
};

class $modify(GJBaseGameLayer) {
	/*
	void update(float dt) {
		GJBaseGameLayer::update(dt);
		//log::info("GJBaseGameLayer::update");
		g_mutex.lock();
		if (zcblive_set_playlayer_time) {
			double time = *(double*)((char*)PlayLayer::get() + 0x328);
			zcblive_set_playlayer_time(time);
		}
		g_mutex.unlock();
	}
	*/

	void handleButton(bool holding, int button, bool player1) {
		GJBaseGameLayer::handleButton(holding, button, player1);
		log::info("GJBaseGameLayer::handleButton");
		g_mutex.lock();
		if (zcblive_set_playlayer_time) {
			PlayLayer* pl = PlayLayer::get();
			double time = pl ? (*(double*)((char*)pl + 0x328)) : 0.0;
			zcblive_set_playlayer_time(time);
		}
		if (zcblive_on_action) {
			zcblive_on_action(holding ? PB_PUSH : PB_RELEASE, !player1);
		}
		g_mutex.unlock();
	}

	bool init() {
		bool res = GJBaseGameLayer::init();
		log::info("GJBaseGameLayer::init");
		g_mutex.lock();
		if (zcblive_on_playlayer_init) {
			zcblive_on_playlayer_init();
		}
		if (zcblive_set_is_in_level) {
			zcblive_set_is_in_level(true);
		}
		g_mutex.unlock();
		return res;
	}
};

class $modify(PlayLayer) {
	void resetLevel() {
		PlayLayer::resetLevel();
		log::info("PlayLayer::resetLevel");
		g_mutex.lock();
		if (zcblive_on_basegamelayer_reset) {
			zcblive_on_basegamelayer_reset();
		}
		if (zcblive_set_is_in_level) {
			zcblive_set_is_in_level(true);
		}
		g_mutex.unlock();
	}
};
