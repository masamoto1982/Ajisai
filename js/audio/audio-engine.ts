// js/audio/audio-engine.ts
//
// 【責務】
// 音楽DSLのフロントエンド再生エンジン。
// SEQ/SIM構造を再帰的に処理し、Web Audio APIで音声を再生する。
//
// 【AudioStructure形式】
// - tone: { type: 'tone', frequency: number, duration: number }
// - rest: { type: 'rest', duration: number }
// - seq: { type: 'seq', children: AudioStructure[] }
// - sim: { type: 'sim', children: AudioStructure[] }

interface AudioStructure {
    type: 'tone' | 'rest' | 'seq' | 'sim';
    frequency?: number;
    duration?: number;
    children?: AudioStructure[];
}

interface PlayCommand {
    type: 'play';
    structure: AudioStructure;
}

// Legacy format support (for backwards compatibility)
interface LegacyAudioNote {
    type: 'single' | 'chord' | 'rest' | 'duration_marker';
    frequency?: number;
    frequencies?: number[];
    duration: 'short' | 'normal' | 'long';
    long?: boolean;
}

interface LegacyAudioTrack {
    track: number;
    notes: LegacyAudioNote[];
}

interface LegacyAudioCommand {
    type: 'sound';
    tracks: LegacyAudioTrack[];
}

type AudioCommand = PlayCommand | LegacyAudioCommand;

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

    async playAudioCommand(command: AudioCommand): Promise<void> {
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

        // Check command type
        if (command.type === 'play') {
            // New format: play with AudioStructure
            const startTime = this.audioContext.currentTime;
            await this.playStructure(command.structure, startTime);
        } else if (command.type === 'sound') {
            // Legacy format: play all tracks simultaneously
            const promises = command.tracks.map(track => this.playLegacyTrack(track));
            await Promise.all(promises);
        }
    }

    // ============================================================================
    // New AudioStructure playback
    // ============================================================================

    private async playStructure(structure: AudioStructure, startTime: number): Promise<number> {
        if (!this.audioContext) return startTime;

        switch (structure.type) {
            case 'tone':
                const toneDuration = (structure.duration || 1) * this.slotDuration;
                this.playTone(structure.frequency!, startTime, toneDuration);
                return startTime + toneDuration;

            case 'rest':
                const restDuration = (structure.duration || 1) * this.slotDuration;
                return startTime + restDuration;

            case 'seq':
                // Sequential: play children one after another
                let seqTime = startTime;
                for (const child of structure.children || []) {
                    seqTime = await this.playStructure(child, seqTime);
                }
                return seqTime;

            case 'sim':
                // Simultaneous: play all children at the same time
                const endTimes = await Promise.all(
                    (structure.children || []).map(child =>
                        this.playStructure(child, startTime)
                    )
                );
                return Math.max(...endTimes, startTime);

            default:
                console.warn('Unknown structure type:', (structure as any).type);
                return startTime;
        }
    }

    // ============================================================================
    // Legacy track playback (backwards compatibility)
    // ============================================================================

    private async playLegacyTrack(track: LegacyAudioTrack): Promise<void> {
        if (!this.audioContext) return;

        console.log(`Playing legacy track ${track.track} with ${track.notes.length} notes`);

        const stepDuration = this.slotDuration;
        let currentTime = this.audioContext.currentTime;

        for (const note of track.notes) {
            const duration = this.getLegacyNoteDuration(note.duration, stepDuration);

            console.log('Processing note:', note);

            if (note.type === 'single' && note.frequency) {
                console.log(`Playing single tone: ${note.frequency}Hz for ${duration}s at time ${currentTime}`);
                this.playTone(note.frequency, currentTime, duration);
            } else if (note.type === 'chord' && note.frequencies) {
                console.log(`Playing chord: ${note.frequencies.join(', ')}Hz for ${duration}s at time ${currentTime}`);
                for (const freq of note.frequencies) {
                    this.playTone(freq, currentTime, duration);
                }
            }

            currentTime += stepDuration;
        }
    }

    private getLegacyNoteDuration(duration: string, stepDuration: number): number {
        switch (duration) {
            case 'short': return stepDuration * 0.5;
            case 'long': return stepDuration * 2;
            case 'normal':
            default: return stepDuration;
        }
    }

    // ============================================================================
    // Core tone generation
    // ============================================================================

    private playTone(frequency: number, startTime: number, duration: number): void {
        if (!this.audioContext) return;

        console.log(`Creating oscillator: ${frequency}Hz, start: ${startTime}, duration: ${duration}`);

        const oscillator = this.audioContext.createOscillator();
        const gainNode = this.audioContext.createGain();

        oscillator.connect(gainNode);
        gainNode.connect(this.audioContext.destination);

        oscillator.frequency.setValueAtTime(frequency, startTime);
        oscillator.type = 'sine';

        // Envelope for smooth sound
        gainNode.gain.setValueAtTime(0, startTime);
        gainNode.gain.linearRampToValueAtTime(0.3, startTime + 0.01);
        gainNode.gain.exponentialRampToValueAtTime(0.01, startTime + duration);

        oscillator.start(startTime);
        oscillator.stop(startTime + duration);

        console.log(`Oscillator started for ${frequency}Hz`);
    }

    // ============================================================================
    // Configuration
    // ============================================================================

    setSlotDuration(seconds: number): void {
        this.slotDuration = seconds;
    }

    getSlotDuration(): number {
        return this.slotDuration;
    }
}

export const AUDIO_ENGINE = new AudioEngine();
