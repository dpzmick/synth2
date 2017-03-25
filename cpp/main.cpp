#define _USE_MATH_DEFINES
#include <cmath>
#include <jack/jack.h>
#include <jack/midiport.h>
#include <unistd.h>

#include <experimental/optional>
#include <functional>
#include <iostream>
#include <limits>

class SineWaveGenerator;
class SineWaveNote;

template<class Generator>
struct GeneratorTraits { };

template<>
struct GeneratorTraits<SineWaveGenerator> {
  using note_type = SineWaveNote;
};

class SineWaveNote {
public:
  SineWaveNote(float freq, float vel)
  : phase_(0)
  , frequency_(freq)
  , velocity_(vel)
  , on_(true)
  { }

  // only added so that the harmonic note could construct this
  SineWaveNote()
  : SineWaveNote(0.0, 0.0)
  { }

  bool is_on() { return on_; }
  void turn_on() { on_ = true; }
  void turn_off() { on_ = false; }
  float get_frequency() { return frequency_; }

private:
  size_t phase_;
  float frequency_;
  float velocity_;
  bool on_;

  friend SineWaveGenerator;
};

class SineWaveGenerator {
public:
  using Traits = GeneratorTraits<SineWaveGenerator>;
  using Note = Traits::note_type;

  SineWaveGenerator(float sample_rate)
  : srate_(sample_rate)
  { }

  float generate(Note& note) {
    float c = srate_ / note.frequency_;

    if (note.phase_ > c) {
      note.phase_ = 1;
    } else {
      note.phase_ += 1;
    }

    if (!note.on_) {
      note.velocity_ /= decay_;
    }

    float x = 2.0 * M_PI * (note.frequency_/srate_ * note.phase_);
    return note.velocity_ * sin(x);
  }

  bool is_note_dead(Note& note) {
    return !note.on_ && note.velocity_ < 0.01;
  }

private:
  float decay_ = 1.05;
  float srate_;
};

template <typename WaveGenerator, int... Harmonics>
class HarmonicGenerator;

template<typename Note, int Harmonic, int... Harmonics>
struct HarmonicNote;

template<typename Generator, int... Harmonics>
struct GeneratorTraits<HarmonicGenerator<Generator, Harmonics...>> {
  using subnote_type = typename GeneratorTraits<Generator>::note_type;
  using note_type = HarmonicNote<subnote_type, Harmonics...>;
};

template <typename Generator, int... Harmonics>
class HarmonicGenerator {
public:
  using Traits = GeneratorTraits<HarmonicGenerator>;
  using Note = typename Traits::note_type;

  HarmonicGenerator(float sample_rate)
  : gen_(sample_rate)
  { }

  float generate(Note& note) {
    float sum = 0.0;
    float constant = 1.0 / sizeof...(Harmonics);

    for (auto& subnote : note) {
      sum += constant * gen_.generate(subnote);
    }

    return sum;
  }

  bool is_note_dead(Note& note) {
    for (auto& subnote : note) {
      if (!gen_.is_note_dead(subnote)) return false;
    }

    return true;
  }

private:
  Generator gen_;
};

template<typename Note, int Harmonic, int... Harmonics>
struct HarmonicNote {
  using Storage = std::array<Note, 1 + sizeof...(Harmonics)>;

  HarmonicNote(float freq, float vel) {
    std::initializer_list<int> hs = {Harmonic, Harmonics...};
    size_t index = 0;
    for (int h : hs) {
      new (&notes_[index++]) Note(freq * h, vel);
    }
  }

  bool is_on() { return notes_[0].is_on(); }

  void turn_on() {
    for (auto& subnote : notes_) {
      subnote.turn_on();
    }
  }

  void turn_off() {
    for (auto& subnote : notes_) {
      subnote.turn_off();
    }
  }

  float get_frequency() {
    return notes_[0].get_frequency();
  }

  typename Storage::iterator begin() { return notes_.begin(); }
  typename Storage::iterator end() { return notes_.end(); }

private:
  Storage notes_;
};

template<int N, typename NoteType>
class EventManager {
public:
  using Storage = std::array<std::experimental::optional<NoteType>, N>;

  typename Storage::iterator begin() { return std::begin(events_); }
  typename Storage::iterator end() { return std::end(events_); }

  void note_on(NoteType note) {
    next_free() = note;
  }

  void note_off(float freq) {
    for (auto& n: events_) {
      if (n && n->is_on() && n->get_frequency() == freq) {
        n->turn_off();
      }
    }
  }

  void kill_note(std::experimental::optional<NoteType>& note) {
    std::cout << "kill note" << std::endl;
    note = std::experimental::nullopt;
  }

private:
  std::experimental::optional<NoteType>& next_free() {
    for (auto& n : events_) {
      if (!n) return n;
    }

    abort();
  }

  Storage events_;
};

float midi_note_to_frequency(char note) {
  return (440.0 / 32.0) * pow(2.0, (note - 9.0) / 12.0);
}

float midi_velocity_to_velocity(char vel) {
  return vel / std::numeric_limits<char>::max();
}

template<typename Generator>
class AudioHandler {
public:
  AudioHandler(jack_port_t* in, jack_port_t* out, float sample_rate)
  : gen_(sample_rate)
  , in_(in)
  , out_(out)
  { }

  int process(jack_nframes_t nframes) {
    jack_default_audio_sample_t* out
      = reinterpret_cast<jack_default_audio_sample_t*>(jack_port_get_buffer(out_, nframes));
    if (!out) {
      abort();
    }

    void* in = jack_port_get_buffer(in_, nframes);
    if (!in) {
      abort();
    }

    jack_midi_event_t current_event;
    uint32_t current_event_idx = 0;
    uint32_t event_count = jack_midi_get_event_count(in);
    if (event_count) {
      std::cout << "event_count: " << event_count << std::endl;
    }

    for (jack_nframes_t i = 0; i < nframes; ++i) {
      // get all the midi events which are relevant for this frame
      while (current_event_idx < event_count) {
        int ret = jack_midi_event_get(&current_event, in, current_event_idx);
        if (ret != 0) {
          abort();
        }

        if (current_event.time != i) break;
        current_event_idx += 1;

        std::cout << "tag: " << std::hex << (int)current_event.buffer[0] << std::endl;
        switch (current_event.buffer[0]) {
          case 0x80: {
            std::cout << "note off" << std::endl;
            float freq = midi_note_to_frequency(current_event.buffer[1]);
            ev_.note_off(freq);
            break;
          }

          case 0x90: {
            std::cout << "note on" << std::endl;
            float freq = midi_note_to_frequency(current_event.buffer[1]);
            float vel = midi_velocity_to_velocity(current_event.buffer[2]);
            vel = 1.0;

            // create a new note and copy it into the event manager
            typename Traits::note_type note(freq, vel);
            ev_.note_on(note);
            break;
          }
        }
      }

      // generate the frame
      jack_default_audio_sample_t frame = 0.0;

      for (auto& event: ev_) {
        if (event) {
          frame += gen_.generate(*event);

          if (gen_.is_note_dead(*event)) {
            ev_.kill_note(event);
          }
        }
      }

      out[i] = frame;
    }

    return 0;
  }

private:
  using Traits = GeneratorTraits<Generator>;

  Generator gen_;
  EventManager<2048, typename Traits::note_type> ev_;

  jack_port_t* in_;
  jack_port_t* out_;
};

template<typename Handler>
int jack_process_handler(jack_nframes_t nframes, void* arg) {
  Handler* gen = reinterpret_cast<Handler*>(arg);
  return gen->process(nframes);
}

int main() {
  jack_status_t status;
  jack_options_t opts = (jack_options_t)(JackNoStartServer | JackUseExactName);
  jack_client_t* c = jack_client_open("sine", opts, &status);

  jack_port_t* input = jack_port_register(
      c, "in", JACK_DEFAULT_MIDI_TYPE, JackPortIsInput, 0);

  if (!input) {
    abort();
  }
  std::cout << "created input port: " << input << std::endl;

  jack_port_t* output = jack_port_register(
      c, "out", JACK_DEFAULT_AUDIO_TYPE, JackPortIsOutput, 0);

  if (!output) {
    abort();
  }
  std::cout << "created output port: " << output << std::endl;

  AudioHandler<HarmonicGenerator<SineWaveGenerator, 1, 2, 3, 4>> handler(input, output, 44100.0);
  jack_set_process_callback(c, jack_process_handler<decltype(handler)>, &handler);

  if (0 != jack_activate(c)) abort();

  while (1) {
    usleep(1000);
  }
}
