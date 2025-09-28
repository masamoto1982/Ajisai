// js/audio/audio-engine.ts

interface AudioNote {
    type: 'single' | 'chord' | 'rest' | 'duration_marker';
    frequency?: number;
    frequencies?: number[];
    duration: 'short' | 'normal' | 'long';
    long?: boolean;
}

interface AudioTrack {
    track: number;
    notes: AudioNote[];
}

interface AudioCommand {
    type: 'sound';
    tracks: AudioTrack[];
}

export class AudioEngine {
    private audioContext: AudioContext | null = null;
    private isInitialized = false;

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

        // Play all tracks simultaneously
        const promises = command.tracks.map(track => this.playTrack(track));
        await Promise.all(promises);
    }

    private async playTrack(track: AudioTrack): Promise<void> {
        if (!this.audioContext) return;

        const stepDuration = 0.5; // 0.5 seconds per step
        let currentTime = this.audioContext.currentTime;

        for (const note of track.notes) {
            const duration = this.getNoteDuration(note.duration, stepDuration);
            
            if (note.type === 'single' && note.frequency) {
                this.playTone(note.frequency, currentTime, duration);
            } else if (note.type === 'chord' && note.frequencies) {
                for (const freq of note.frequencies) {
                    this.playTone(freq, currentTime, duration);
                }
            }
            // 'rest' and 'duration_marker' types don't produce sound
            
            currentTime += stepDuration;
        }
    }

    private getNoteDuration(duration: string, stepDuration: number): number {
        switch (duration) {
            case 'short': return stepDuration * 0.5;
            case 'long': return stepDuration * 2;
            case 'normal':
            default: return stepDuration;
        }
    }

    private playTone(frequency: number, startTime: number, duration: number): void {
        if (!this.audioContext) return;

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
    }

    // Test function
    async playTestTone(): Promise<void> {
        await this.init();
        if (!this.audioContext) return;

        this.playTone(440, this.audioContext.currentTime, 1.0);
        console.log('Test tone played: 440Hz for 1 second');
    }
}

export const AUDIO_ENGINE = new AudioEngine();
