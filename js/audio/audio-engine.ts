// js/audio/audio-engine.ts
//
// Audio playback engine for Ajisai's music DSL.
// Recursively processes SEQ/SIM structures and plays audio via Web Audio API.
//
// AudioStructure format:
// - tone: { type: 'tone', frequency: number, duration: number, envelope?, waveform? }
// - rest: { type: 'rest', duration: number }
// - seq: { type: 'seq', children: AudioStructure[], envelope?, waveform? }
// - sim: { type: 'sim', children: AudioStructure[], envelope?, waveform? }

type WaveformType = 'sine' | 'square' | 'sawtooth' | 'triangle';

interface Envelope {
    attack: number;   // 立ち上がり時間（秒）
    decay: number;    // 減衰時間（秒）
    sustain: number;  // 持続レベル（0.0-1.0）
    release: number;  // 余韻時間（秒）
}

interface AudioStructure {
    type: 'tone' | 'rest' | 'seq' | 'sim';
    frequency?: number;
    duration?: number;
    children?: AudioStructure[];
    envelope?: Envelope;
    waveform?: WaveformType;
}

interface PlayCommand {
    type: 'play';
    structure: AudioStructure;
}

// デフォルトエンベロープ
const DEFAULT_ENVELOPE: Envelope = {
    attack: 0.01,
    decay: 0.0,
    sustain: 1.0,
    release: 0.01,
};

export class AudioEngine {
    private audioContext: AudioContext | null = null;
    private isInitialized = false;
    private slotDuration = 0.5; // 0.5 seconds per slot

    async init(): Promise<void> {
        if (this.isInitialized) return;

        try {
            this.audioContext = new (window.AudioContext || (window as any).webkitAudioContext)();
            this.isInitialized = true;
            console.log('Audio engine initialized');
        } catch (error) {
            console.error('Failed to initialize audio engine:', error);
        }
    }

    async playAudioCommand(command: PlayCommand): Promise<void> {
        if (!this.audioContext) {
            await this.init();
        }

        if (!this.audioContext) {
            console.error('Audio context not available');
            return;
        }

        // Resume audio context if suspended (required by some browsers)
        if (this.audioContext.state === 'suspended') {
            await this.audioContext.resume();
        }

        const startTime = this.audioContext.currentTime;
        await this.playStructure(command.structure, startTime);
    }

    private async playStructure(
        structure: AudioStructure,
        startTime: number,
        inheritedEnvelope?: Envelope,
        inheritedWaveform?: WaveformType
    ): Promise<number> {
        if (!this.audioContext) return startTime;

        // 継承されたパラメータまたはデフォルトを使用
        const envelope = structure.envelope || inheritedEnvelope || DEFAULT_ENVELOPE;
        const waveform = structure.waveform || inheritedWaveform || 'sine';

        switch (structure.type) {
            case 'tone':
                const toneDuration = (structure.duration || 1) * this.slotDuration;
                this.playTone(structure.frequency!, startTime, toneDuration, envelope, waveform);
                return startTime + toneDuration;

            case 'rest':
                const restDuration = (structure.duration || 1) * this.slotDuration;
                return startTime + restDuration;

            case 'seq':
                // Sequential: play children one after another
                let seqTime = startTime;
                for (const child of structure.children || []) {
                    seqTime = await this.playStructure(child, seqTime, envelope, waveform);
                }
                return seqTime;

            case 'sim':
                // Simultaneous: play all children at the same time
                const endTimes = await Promise.all(
                    (structure.children || []).map(child =>
                        this.playStructure(child, startTime, envelope, waveform)
                    )
                );
                return Math.max(...endTimes, startTime);

            default:
                console.warn('Unknown structure type:', (structure as any).type);
                return startTime;
        }
    }

    private playTone(
        frequency: number,
        startTime: number,
        duration: number,
        envelope: Envelope,
        waveform: WaveformType
    ): void {
        if (!this.audioContext) return;

        const oscillator = this.audioContext.createOscillator();
        const gainNode = this.audioContext.createGain();

        oscillator.connect(gainNode);
        gainNode.connect(this.audioContext.destination);

        oscillator.frequency.setValueAtTime(frequency, startTime);
        oscillator.type = waveform;

        // ADSR Envelope
        const { attack, decay, sustain, release } = envelope;
        const peakLevel = 0.3;  // 最大音量
        const sustainLevel = peakLevel * sustain;

        // リリースを考慮した実効音長
        const effectiveDuration = Math.max(duration - release, attack + decay);

        // Attack: 0 → peakLevel
        gainNode.gain.setValueAtTime(0, startTime);
        gainNode.gain.linearRampToValueAtTime(peakLevel, startTime + attack);

        // Decay: peakLevel → sustainLevel
        if (decay > 0) {
            gainNode.gain.linearRampToValueAtTime(sustainLevel, startTime + attack + decay);
        }

        // Sustain: sustainLevel を維持
        // Release: sustainLevel → 0
        const releaseStart = startTime + effectiveDuration;
        gainNode.gain.setValueAtTime(sustainLevel, releaseStart);
        gainNode.gain.exponentialRampToValueAtTime(0.001, releaseStart + release);

        oscillator.start(startTime);
        oscillator.stop(startTime + duration + release);
    }

    setSlotDuration(seconds: number): void {
        this.slotDuration = seconds;
    }

    getSlotDuration(): number {
        return this.slotDuration;
    }
}

export const AUDIO_ENGINE = new AudioEngine();
