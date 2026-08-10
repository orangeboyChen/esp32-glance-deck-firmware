import { atom } from 'jotai'

export type CommandPhase = 'idle' | 'submitting' | 'accepted' | 'error'

export type CommandFeedback = {
  device_id: string
  message: string
  phase: CommandPhase
}

export const selected_device_id_atom = atom<string | null>(null)
export const selected_preview_id_atom = atom<string | null>(null)
export const command_feedback_atom = atom<CommandFeedback | null>(null)

export const begin_device_command_atom = atom(
  null,
  (_get, set, device_id: string) => {
    set(command_feedback_atom, {
      device_id,
      message: '',
      phase: 'submitting',
    })
  },
)

export const resolve_device_command_atom = atom(
  null,
  (_get, set, feedback: CommandFeedback) => set(command_feedback_atom, feedback),
)
