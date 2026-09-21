import React from 'react'
import { useNotesPage } from '../hooks/useNotesPage'
import { TeamOverview } from './TeamOverview'
import { NoteList } from './NoteList'
import { NoteReadPane } from './NoteReadPane'
import { NoteComposePane } from './NoteComposePane'

export const NotesPage: React.FC = () => {
  const {
    user,
    userRole,
    teams,
    selectedTeam,
    notes,
    activeNote,
    isComposing,
    searchQuery,
    setSearchQuery,
    noteSearchQuery,
    setNoteSearchQuery,
    composeText,
    setComposeText,
    composeSubject,
    setComposeSubject,
    expandedAudit,
    setExpandedAudit,
    activeNoteId,
    setActiveNoteId,
    setSelectedTeamId,
    setIsComposing,
    handleSaveNote,
    handleDeleteNote,
    handleEditNote,
    handleSelectTeam,
    startComposing,
    unreadNotes,
  } = useNotesPage()

  return (
    <div className="flex h-full w-full flex-1 flex-col bg-background">
      {isComposing ? (
        <NoteComposePane
          selectedTeam={selectedTeam}
          setIsComposing={setIsComposing}
          composeSubject={composeSubject}
          setComposeSubject={setComposeSubject}
          composeText={composeText}
          setComposeText={setComposeText}
          handleSaveNote={handleSaveNote}
        />
      ) : activeNoteId ? (
        <NoteReadPane
          activeNote={activeNote}
          setActiveNoteId={setActiveNoteId}
          handleEditNote={handleEditNote}
          handleDeleteNote={handleDeleteNote}
          currentUser={user?.id}
          userRole={userRole}
          expandedAudit={expandedAudit}
          setExpandedAudit={setExpandedAudit}
        />
      ) : selectedTeam ? (
        <NoteList
          selectedTeam={selectedTeam}
          notes={notes}
          noteSearchQuery={noteSearchQuery}
          setNoteSearchQuery={setNoteSearchQuery}
          setSelectedTeamId={setSelectedTeamId}
          setActiveNoteId={setActiveNoteId}
          startComposing={startComposing}
          handleEditNote={handleEditNote}
          handleDeleteNote={handleDeleteNote}
          currentUser={user?.id}
          userRole={userRole}
        />
      ) : (
        <TeamOverview
          teams={teams}
          searchQuery={searchQuery}
          setSearchQuery={setSearchQuery}
          unreadNotes={unreadNotes}
          handleSelectTeam={handleSelectTeam}
        />
      )}
    </div>
  )
}
