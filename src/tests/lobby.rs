//! the multiplayer form and its lobbies: what typing does to a field, what the form asks for and what it says is wrong, what a joined player is told

use super::*;

fn typed(field: &mut TextField, text: &str) {
    for letter in text.chars() {
        field.apply(Edit::Type(letter));
    }
}

fn field(allowed: Allowed, limit: usize) -> TextField {
    TextField::new("", limit, allowed)
}

fn member(slot: u8, name: &str) -> Member {
    Member { slot, name: name.to_owned(), ping_ms: 20 }
}

// ---------- a line of text ----------

#[test]
fn typing_puts_letters_in_at_the_cursor() {
    let mut name = field(Allowed::Name, 16);
    typed(&mut name, "Anne");
    assert_eq!((name.text(), name.cursor()), ("Anne", 4));
    name.apply(Edit::Left);
    name.apply(Edit::Left);
    name.apply(Edit::Type('X'));
    assert_eq!((name.text(), name.cursor()), ("AnXne", 3));
    name.apply(Edit::Home);
    name.apply(Edit::Type('-'));
    assert_eq!(name.text(), "-AnXne");
    name.apply(Edit::End);
    name.apply(Edit::Type('!'));
    assert_eq!(name.text(), "-AnXne", "an exclamation mark is not a letter of a name");
}

#[test]
fn backspace_and_delete_take_out_letters_and_do_nothing_at_the_ends() {
    let mut text = TextField::new("abcd", 10, Allowed::Password);
    text.apply(Edit::Delete);
    assert_eq!(text.text(), "abcd", "nothing after the cursor");
    text.apply(Edit::Backspace);
    assert_eq!((text.text(), text.cursor()), ("abc", 3));
    text.apply(Edit::Home);
    text.apply(Edit::Backspace);
    assert_eq!(text.text(), "abc", "nothing before the cursor");
    text.apply(Edit::Delete);
    assert_eq!((text.text(), text.cursor()), ("bc", 0));
    for _ in 0..5 {
        text.apply(Edit::Right);
    }
    assert_eq!(text.cursor(), 2, "the cursor stops at the end");
    for _ in 0..5 {
        text.apply(Edit::Left);
    }
    assert_eq!(text.cursor(), 0, "and at the start");
}

#[test]
fn a_field_holds_no_more_than_its_limit() {
    let mut name = field(Allowed::Name, 5);
    typed(&mut name, "abcdefgh");
    assert_eq!(name.text(), "abcde");
    name.apply(Edit::Paste("xyz".to_owned()));
    assert_eq!(name.text(), "abcde");
    //(it is letters that count, not bytes)
    let mut name = field(Allowed::Name, 3);
    typed(&mut name, "éèà!");
    assert_eq!(name.text(), "éèà");
    assert_eq!(name.len(), 3);
}

#[test]
fn letters_with_accents_are_edited_as_one_letter() {
    let mut text = TextField::new("naïve", 20, Allowed::Password);
    text.apply(Edit::Left);
    text.apply(Edit::Left);
    text.apply(Edit::Left);
    text.apply(Edit::Backspace);
    assert_eq!(text.text(), "nïve", "the a before the ï was taken out");
    text.apply(Edit::Delete);
    assert_eq!(text.text(), "nve");
}

#[test]
fn each_kind_of_field_takes_only_its_own_letters() {
    let mut name = field(Allowed::Name, 30);
    typed(&mut name, "Ann-Marie O'Neil_2.0 <b>");
    assert_eq!(name.text(), "Ann-Marie O'Neil_2.0 b", "marks that can't be in a name are left out");
    let mut port = field(Allowed::Port, 5);
    typed(&mut port, "4a7b8-21");
    assert_eq!(port.text(), "47821");
    let mut password = field(Allowed::Password, 30);
    typed(&mut password, "pass word! \u{7}");
    assert_eq!(password.text(), "pass word! ", "anything but what can't be seen");
}

#[test]
fn a_paste_is_one_line_cleaned_and_cut_to_fit() {
    let mut password = field(Allowed::Password, 8);
    password.apply(Edit::Paste("secret\r\nsecond line".to_owned()));
    assert_eq!(password.text(), "secret", "only the first line");
    password.apply(Edit::Paste("abcdef".to_owned()));
    assert_eq!(password.text(), "secretab", "and no more than fits");
    //(into the middle of a line)
    let mut name = TextField::new("Anne", 16, Allowed::Name);
    name.apply(Edit::Home);
    name.apply(Edit::Right);
    name.apply(Edit::Paste("lex".to_owned()));
    assert_eq!((name.text(), name.cursor()), ("Alexnne", 4));
}

#[test]
fn a_field_that_is_given_text_to_start_with_takes_what_it_would_have_been_typed() {
    assert_eq!(TextField::new("a\tb c", 10, Allowed::Name).text(), "ab c");
    assert_eq!(TextField::new("toolongforthis", 4, Allowed::Name).text(), "tool");
    assert_eq!(TextField::new("12ab34", 5, Allowed::Port).cursor(), 4, "the cursor starts at the end");
}

#[test]
fn a_secret_is_shown_as_marks() {
    let text = TextField::new("hunter22", 20, Allowed::Password);
    assert_eq!(text.shown(true), "********");
    assert_eq!(text.shown(false), "hunter22");
    assert_eq!(TextField::new("naïve", 20, Allowed::Password).shown(true), "*****", "a mark for each letter, not each byte");
}

// ---------- the form ----------

fn form_with(name: &str, password: &str) -> Form {
    let mut form = Form::new(&Settings::default());
    form.name = TextField::new(name, MAX_NAME_LENGTH, Allowed::Name);
    form.password = TextField::new(password, MAX_PASSWORD, Allowed::Password);
    form
}

#[test]
fn the_form_starts_with_what_the_player_used_last_time_except_the_password() {
    let fresh = Form::new(&Settings::default());
    assert_eq!(fresh.role, Role::Host, "someone who has never joined is more likely to host");
    assert_eq!(fresh.focus, Row::Name, "with no name to go on they start by giving one");
    assert_eq!(fresh.port.text(), DEFAULT_PORT.to_string());
    let kept = Settings { player_name: "Ann".to_owned(), host_port: 50000, multiplayer_role: Role::Local, ..Settings::default() };
    let form = Form::new(&kept);
    assert_eq!(form.role, Role::Local, "what they did last time");
    assert_eq!(form.focus, Row::Password, "the name is there already");
    assert_eq!((form.name.text(), form.port.text()), ("Ann", "50000"));
    assert_eq!(form.password.text(), "", "the password is not kept");
    //(someone who joins from the list has no password to type yet, and starts on the name)
    let joiner = Form::new(&Settings { player_name: "Ann".to_owned(), multiplayer_role: Role::Join, ..Settings::default() });
    assert_eq!((joiner.role, joiner.focus), (Role::Join, Row::Name));
}

#[test]
fn tab_and_the_arrows_go_round_the_rows() {
    let mut form = Form::new(&Settings::default());
    form.focus = Row::Role;
    form.step(1);
    assert_eq!(form.focus, Row::Name);
    form.step(1);
    form.step(1);
    assert_eq!(form.focus, Row::Port);
    form.step(1);
    assert_eq!(form.focus, Row::Role, "round to the start");
    form.step(-1);
    assert_eq!(form.focus, Row::Port, "and back");
    //(someone joining from the list has only the name to give, and is not left on what is not there)
    form.switch(Role::Join);
    assert_eq!(form.focus, Row::Name);
    form.step(1);
    assert_eq!(form.focus, Row::Role);
    form.step(-1);
    assert_eq!(form.focus, Row::Name);
}

#[test]
fn enter_moves_on_until_the_last_row_and_then_asks_to_go() {
    let mut form = Form::new(&Settings::default());
    form.focus = Row::Role;
    assert!(!form.enter());
    assert_eq!(form.focus, Row::Name);
    assert!(!form.enter());
    assert!(!form.enter());
    assert_eq!(form.focus, Row::Port);
    assert!(form.enter(), "on the port it goes ahead");
    assert_eq!(form.focus, Row::Port);
    //(for someone who joins from the list the name is the last row)
    form.switch(Role::Join);
    assert!(form.enter());
}

#[test]
fn the_roles_go_round_and_are_known_by_their_names() {
    assert_eq!(Role::Host.next(), Role::Join);
    assert_eq!(Role::Join.next(), Role::Local);
    assert_eq!(Role::Local.next(), Role::Host, "round to the first");
    assert_eq!((Role::Host.before(), Role::Join.before(), Role::Local.before()), (Role::Host, Role::Host, Role::Join), "and stops at the first going back");
    assert_eq!((Role::Host.after(), Role::Join.after(), Role::Local.after()), (Role::Join, Role::Local, Role::Local), "and the last going on");
    for role in [Role::Host, Role::Join, Role::Local] {
        assert_eq!(Role::from_name(role.name()), Some(role));
    }
    assert_eq!(Role::from_name("somewhere"), None);
}

#[test]
fn the_fourth_row_is_the_port_and_a_joiner_from_the_list_has_no_port_or_password() {
    let mut form = Form::new(&Settings::default());
    form.focus = Row::Port;
    form.field(Row::Port).unwrap().apply(Edit::End);
    for role in [Role::Host, Role::Local] {
        form.switch(role);
        assert_eq!(rows(role), [Row::Role, Row::Name, Row::Password, Row::Port]);
        assert_eq!(form.field(Row::Port).unwrap().text(), DEFAULT_PORT.to_string());
    }
    assert_eq!(rows(Role::Join), [Row::Role, Row::Name], "the password is asked for when a game is picked");
    assert!(form.field(Row::Role).is_none());
}

#[test]
fn the_form_says_what_is_wrong_in_the_order_the_player_sees_it() {
    let Err(problem) = form_with("", PASSWORD_FOR_FORMS).submit() else { panic!("no name") };
    assert!(problem.contains("name"), "{problem}");
    let Err(problem) = form_with("   ", PASSWORD_FOR_FORMS).submit() else { panic!("a name of nothing") };
    assert!(problem.contains("name"), "{problem}");
    let Err(problem) = form_with("Ann", "abc").submit() else { panic!("a short password") };
    assert!(problem.contains("password"), "{problem}");
    let mut form = form_with("Ann", PASSWORD_FOR_FORMS);
    form.port = TextField::new("80", 5, Allowed::Port);
    let Err(problem) = form.submit() else { panic!("a port for other programs") };
    assert!(problem.contains("1024"), "{problem}");
    form.switch(Role::Local);
    let Err(problem) = form.submit() else { panic!("a port for other programs, to join as well") };
    assert!(problem.contains("1024"), "{problem}");
    //(a joiner from the list is asked for a name, and nothing else yet)
    form.switch(Role::Join);
    assert!(form.submit().is_ok());
    form.name = TextField::new("", MAX_NAME_LENGTH, Allowed::Name);
    let Err(problem) = form.submit() else { panic!("no name") };
    assert!(problem.contains("name"), "{problem}");
}

const PASSWORD_FOR_FORMS: &str = "open sesame";

#[test]
fn a_form_filled_in_right_asks_to_host_on_the_lobby_list() {
    let mut form = form_with("  Ann  ", PASSWORD_FOR_FORMS);
    form.port = TextField::new("50000", 5, Allowed::Port);
    let Ok(Submit::Host { config, list_on }) = form.submit() else { panic!("not a host") };
    assert_eq!(config.name, "Ann", "the name as the game cleans it");
    assert_eq!(config.password, PASSWORD_FOR_FORMS);
    assert_eq!(config.port, 50000);
    assert_eq!(list_on, LOBBY_SERVER, "a game is always on the lobby list, without the player saying so");
}

#[test]
fn a_game_is_not_hosted_when_the_lobby_server_is_no_address() {
    let mut form = form_with("Ann", PASSWORD_FOR_FORMS);
    form.server = " https://army.example.com ".to_owned();
    let Ok(Submit::Host { list_on, .. }) = form.submit() else { panic!("not a host") };
    assert_eq!(list_on, "https://army.example.com");
    form.server = "ftp://army.example.com".to_owned();
    let Err(problem) = form.submit() else { panic!("not an address of a lobby server") };
    assert!(problem.contains("http"), "{problem}");
    //(joining a game on this computer does not go by the lobby list)
    form.switch(Role::Local);
    assert!(form.submit().is_ok());
}

#[test]
fn a_form_filled_in_right_asks_to_see_the_games_to_join() {
    let mut form = form_with("Ann", "");
    form.switch(Role::Join);
    let Ok(Submit::Join { name }) = form.submit() else { panic!("not joining") };
    assert_eq!(name, "Ann", "the password is asked for once a game is picked");
}

#[test]
fn a_game_on_this_computer_is_joined_at_the_loopback_address_and_the_port_it_is_on() {
    let mut form = form_with("Ann", PASSWORD_FOR_FORMS);
    form.switch(Role::Local);
    form.port = TextField::new("50000", 5, Allowed::Port);
    let Ok(Submit::Local { target, name, password }) = form.submit() else { panic!("not joining") };
    assert_eq!(target, "127.0.0.1:50000", "the one address there is, and the player never gives it");
    assert_eq!(name, "Ann");
    assert_eq!(password, PASSWORD_FOR_FORMS);
}

#[test]
fn what_is_worth_keeping_is_kept_once_the_form_is_used() {
    let mut settings = Settings::default();
    let mut form = form_with(" Ann  Marie ", PASSWORD_FOR_FORMS);
    form.port = TextField::new("50000", 5, Allowed::Port);
    form.remember(&mut settings);
    assert_eq!((settings.player_name.as_str(), settings.host_port, settings.multiplayer_role), ("Ann  Marie", 50000, Role::Host));
    form.switch(Role::Join);
    form.remember(&mut settings);
    assert_eq!(settings.multiplayer_role, Role::Join);
    assert!(!settings.to_text().contains(PASSWORD_FOR_FORMS), "the password is never written down");
    //(joining from the list has no port, and a port that is no good is not kept)
    form.port = TextField::new("12", 5, Allowed::Port);
    form.remember(&mut settings);
    assert_eq!(settings.host_port, 50000);
    form.switch(Role::Local);
    form.remember(&mut settings);
    assert_eq!((settings.host_port, settings.multiplayer_role), (50000, Role::Local));
}

// ---------- the settings that go with it ----------

#[test]
fn a_name_with_spaces_in_it_is_read_back_whole() {
    let settings = Settings { player_name: "Ann Marie O'Neil".to_owned(), ..Settings::default() };
    assert_eq!(Settings::from_text(&settings.to_text()).player_name, "Ann Marie O'Neil");
}

#[test]
fn settings_written_without_the_multiplayer_lines_still_read_and_bad_ones_are_ignored() {
    let old = Settings::from_text("master 0.50\ndifficulty Hard\n");
    assert_eq!((old.player_name.as_str(), old.host_port, old.multiplayer_role), ("", DEFAULT_PORT, Role::Host));
    let bad = Settings::from_text("host_port 12\nplayer_name !!!\nhost_port abc\nmultiplayer_role nowhere\n");
    assert_eq!(bad.host_port, DEFAULT_PORT, "a port under 1024, or none, is left alone");
    assert_eq!(bad.player_name, "", "and a name with nothing in it that can be used is nothing");
    assert_eq!(bad.multiplayer_role, Role::Host);
    //(a file from when addresses were typed has lines nobody reads any more, and they are kept nowhere)
    let typed = Settings::from_text("join_address 203.0.113.7:5000\nlist_game no\nlobby_server https://old.example.com\nmultiplayer_role local\n");
    assert_eq!(typed.multiplayer_role, Role::Local);
    let text = typed.to_text();
    assert!(!text.contains("203.0.113.7") && !text.contains("join_address") && !text.contains("list_game") && !text.contains("old.example.com"));
}

// ---------- what a lobby is told ----------

#[test]
fn a_player_who_has_joined_knows_who_is_in_and_that_the_run_started() {
    let mut lobby = Lobby::default();
    assert!(!lobby.joined);
    assert_eq!(lobby.apply(ClientEvent::Joined { slot: 2, members: vec![member(0, "Hostess"), member(2, "Ann")] }), None);
    assert!(lobby.joined);
    assert_eq!(lobby.slot, 2);
    assert_eq!(lobby.members.len(), 2);
    assert_eq!(lobby.apply(ClientEvent::Roster(vec![member(0, "Hostess"), member(1, "Bob"), member(2, "Ann")])), None);
    assert_eq!(lobby.members.len(), 3);
    assert!(!lobby.started);
    assert_eq!(lobby.apply(ClientEvent::Started), None);
    assert!(lobby.started);
}

#[test]
fn what_ends_a_stay_comes_back_as_words_for_the_player() {
    let mut lobby = Lobby::default();
    assert_eq!(lobby.apply(ClientEvent::Refused(Refusal::WrongPassword)).as_deref(), Some("that is not the password"));
    assert_eq!(lobby.apply(ClientEvent::Failed("no answer".to_owned())).as_deref(), Some("no answer"));
    assert_eq!(lobby.apply(ClientEvent::Closed("the connection ended".to_owned())).as_deref(), Some("the connection ended"));
}

#[test]
fn the_host_s_log_says_who_came_and_went_and_who_was_turned_away() {
    let address: std::net::SocketAddr = "203.0.113.9:51000".parse().unwrap();
    let (line, _) = log_line(&HostEvent::Joined { slot: 1, name: "Ann".to_owned(), address });
    assert_eq!(line, "Ann joined");
    let (line, _) = log_line(&HostEvent::Left { slot: 1, name: "Ann".to_owned(), why: "left the game".to_owned() });
    assert_eq!(line, "Ann left (left the game)");
    let (line, color) = log_line(&HostEvent::TurnedAway { address, why: "that is not the password".to_owned() });
    assert_eq!(line, "someone was turned away: that is not the password", "nobody is named by where they came from");
    assert_eq!(color, ALERT);
}

#[test]
fn a_hosted_game_says_where_it_listens_and_who_is_in() {
    assert_eq!(hosting_text(47821, &[member(0, "Hostess")]), "HOSTING ON PORT 47821: Hostess");
    assert_eq!(hosting_text(5000, &[member(0, "Hostess"), member(1, "Ann"), member(3, "Bob")]), "HOSTING ON PORT 5000: Hostess, Ann, Bob");
}
