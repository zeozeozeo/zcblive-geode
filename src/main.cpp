#include <Geode/Geode.hpp>
#include <Geode/modify/GJBaseGameLayer.hpp>
#include <Geode/modify/LevelEditorLayer.hpp>
#include <Geode/modify/PlayLayer.hpp>
#include <Geode/modify/PlayerObject.hpp>

using namespace geode::prelude;

// generated by `cargo rustc -- --print=native-static-libs`
// (i have no idea from why rust wants this much)
// note that there are a lot of duplicates here, Rust says that this is required
#pragma comment(lib, "kernel32.lib")
#pragma comment(lib, "bcrypt.lib")
#pragma comment(lib, "advapi32.lib")
#pragma comment(lib, "legacy_stdio_definitions.lib")
#pragma comment(lib, "advapi32.lib")
#pragma comment(lib, "cfgmgr32.lib")
#pragma comment(lib, "gdi32.lib")
#pragma comment(lib, "kernel32.lib")
#pragma comment(lib, "msimg32.lib")
#pragma comment(lib, "opengl32.lib")
#pragma comment(lib, "shell32.lib")
#pragma comment(lib, "shlwapi.lib")
#pragma comment(lib, "user32.lib")
#pragma comment(lib, "winspool.lib")
#pragma comment(lib, "kernel32.lib")
#pragma comment(lib, "advapi32.lib")
#pragma comment(lib, "kernel32.lib")
#pragma comment(lib, "ntdll.lib")
#pragma comment(lib, "userenv.lib")
#pragma comment(lib, "ws2_32.lib")
#pragma comment(lib, "synchronization.lib")
#pragma comment(lib, "kernel32.lib")
#pragma comment(lib, "ws2_32.lib")
#pragma comment(lib, "kernel32.lib")
#pragma comment(lib, "msvcrt.lib")

// for some reason rust hasn't mentioned these
#pragma comment(lib, "propsys.lib")
#pragma comment(lib, "runtimeobject.lib")

// define the Rust library API (libzcblive is statically linked into the DLL)
extern "C" {
void zcblive_initialize();
void zcblive_uninitialize();
void zcblive_on_action(uint8_t button, bool player2, bool push);
void zcblive_on_reset();
void zcblive_set_is_in_level(bool is_in_level);
void zcblive_set_playlayer_time(double time);
void zcblive_on_init(PlayLayer* playlayer);
void zcblive_on_quit();
void zcblive_on_death();
bool zcblive_do_force_player2_sounds();
bool zcblive_do_use_alternate_hook();
void zcblive_on_update(float dt);
}

// clang-format off
$on_mod(Loaded) {
    // hooks glSwapBuffers (for now), takes panic hook, calls Bot::init
    zcblive_initialize();
}
// clang-format on

// clang-format off
$on_mod(Unloaded) {
	zcblive_uninitialize();
}
// clang-format on

inline double getTime() {
    return PlayLayer::get() ? (*(double*)((char*)PlayLayer::get() + 0x328))
                            : 0.0;
}

void handleAction(int button, bool player1, bool push, PlayLayer* playLayer) {
    // seems to be isPaused?
    if (playLayer &&
        *reinterpret_cast<bool*>(reinterpret_cast<char*>(playLayer) + 0x2f17)) {
        return;
    }

    zcblive_on_action(static_cast<uint8_t>(button),
                      !player1 && playLayer &&
                          (playLayer->m_levelSettings->m_twoPlayerMode ||
                           zcblive_do_force_player2_sounds()),
                      push);
}

// clang-format off
class $modify(PlayerObject) {
	void handlePushOrRelease(PlayerButton button, bool push) {
		auto playLayer = PlayLayer::get();
		if (playLayer == nullptr && LevelEditorLayer::get() == nullptr) {
			zcblive_set_is_in_level(false);
			return;
		}
		if ((button == PlayerButton::Left || button == PlayerButton::Right) && !this->m_isPlatformer) {
			return;
		}

		zcblive_set_is_in_level(true);
		zcblive_set_playlayer_time(getTime());

		bool player1 = playLayer && this == playLayer->m_player1;
		handleAction(static_cast<int>(button), player1, push, playLayer);
	}

	void pushButton(PlayerButton button) {
		if (zcblive_do_use_alternate_hook()) {
			handlePushOrRelease(button, true);
		}
		PlayerObject::pushButton(button);
	}

	void releaseButton(PlayerButton button) {
		if (zcblive_do_use_alternate_hook()) {
			handlePushOrRelease(button, false);
		}
		PlayerObject::releaseButton(button);
	}
};

class $modify(GJBaseGameLayer) {
	void handleButton(bool push, int button, bool player1) {
		if (zcblive_do_use_alternate_hook()) {
			GJBaseGameLayer::handleButton(push, button, player1);
			return;
		}
		zcblive_set_is_in_level(true);
		zcblive_set_playlayer_time(getTime());

		auto playLayer = PlayLayer::get();
		bool is_invalid = playLayer && ((button == 2 || button == 3)
                        && !(player1 && playLayer->m_player1->m_isPlatformer)
                        && !(!player1 && playLayer->m_player2->m_isPlatformer));
		if (!is_invalid) {
			handleAction(button, player1, push, playLayer);
		}
		
		GJBaseGameLayer::handleButton(push, button, player1);
	}

	void update(float dt) {
		zcblive_on_update(dt);
		GJBaseGameLayer::update(dt);
		zcblive_set_playlayer_time(getTime());
	}

	bool init() {
		zcblive_on_init(nullptr); // PlayLayer* could be nullptr on Geode
		return GJBaseGameLayer::init();
	}
};

class $modify(PlayLayer) {
	void onQuit() {
		zcblive_on_quit();
		PlayLayer::onQuit();
	}

	void resetLevel() {
		zcblive_on_reset();
		PlayLayer::resetLevel();
	}

	void destroyPlayer(PlayerObject* player, GameObject* hit) {
		PlayLayer::destroyPlayer(player, hit);
		if (player->m_isDead) {
			zcblive_on_death();
		}
	}
};

class $modify(LevelEditorLayer) {
	bool init(GJGameLevel* level, bool something) {
		zcblive_on_init(nullptr);
		return LevelEditorLayer::init(level, something);
	}
};
// clang-format on
