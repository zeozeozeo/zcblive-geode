#include <Geode/Geode.hpp>
#include <Geode/modify/GJBaseGameLayer.hpp>
#include <Geode/modify/PlayLayer.hpp>

using namespace geode::prelude;

// define the Rust library API (libzcblive is statically linked into the DLL)
extern "C" {
void zcblive_initialize();
void zcblive_on_action(uint8_t button, bool player2, bool push);
void zcblive_set_is_in_level(bool is_in_level);
void zcblive_set_playlayer_time(double time);
void zcblive_on_init(PlayLayer* playlayer);
void zcblive_on_exit();
}

// clang-format off
$on_mod(Loaded) {
    // hooks glSwapBuffers (for now), takes panic hook, calls Bot::init
    zcblive_initialize();
}
// clang-format on

double getTime() {
    return PlayLayer::get() ? (*(double*)((char*)PlayLayer::get() + 0x328))
                            : 0.0;
}

// clang-format off
class $modify(GJBaseGameLayer) {
	void handleButton(bool push, int button, bool player1) {
		zcblive_set_is_in_level(true);
		zcblive_set_playlayer_time(getTime());

		auto playLayer = PlayLayer::get();
		bool is_invalid = (button == 2 || button == 3)
                        && !(player1 && playLayer->m_player1->m_isPlatformer)
                        && !(!player1 && playLayer->m_player2->m_isPlatformer);
		if (!player1 && !playLayer->m_levelSettings->m_twoPlayerMode) {
			is_invalid = true;
		}
		if (!is_invalid) {
			zcblive_on_action(button, !player1, push);
		}
		
		GJBaseGameLayer::handleButton(push, button, player1);
	}

	void update(float dt) {
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
		zcblive_on_exit();
		PlayLayer::onQuit();
	}
};
