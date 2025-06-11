//Bingo with toml/json configuration
use rand;
use serde::Serialize;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::str;
use toml::{Table, Value::Boolean};

fn main() {
    let toml_file = "bingo_config.toml";
    let toml_str = fs::read_to_string(toml_file)
        .expect("Sorry mate, we messed this up and can't find the file");
    let config = toml_str.parse::<Table>().unwrap();
    let card_size = config["board_size"].as_integer().unwrap();
    let number_range = config["number_range"].as_integer().unwrap();
    let row_range = number_range / card_size;
    let number_of_games = config["number_games"].as_integer().unwrap();
    let number_of_players = config["number_players"].as_integer().unwrap();
    let free_space = config["free_space"].as_bool().unwrap();
    let statistics = config["statistics"].as_array().unwrap();
    let verbose = config["verbose"].as_bool().unwrap();
    let output_file = config["output_file"].as_str().unwrap();
    let win_condition: Vec<i64> = vec![0; card_size as usize];

    //Data collection variables
    let counter_average_vector: Vec<f64> = vec![0.0; number_of_players as usize];

    //all_counters holds the total list of all the counters so that the standard deviation can be calculated
    let mut all_counters: Vec<i64> = Vec::new();
    let mut all_called_nums: Vec<Vec<i64>> = vec![vec![]; number_of_players as usize];
    let mut counter_sample_mean: f64 = 0.0;
    let mut horizontal_victory = 0;
    let mut vertical_victory = 0;
    let mut diagonal_victory = 0;

    //statistics variables
    let mut horizontal_victory_percentage = 0.0;
    let mut vertical_victory_percentage = 0.0;
    let mut diagonal_victory_percentage = 0.0;
    let mut player_list: Vec<Vec<Vec<i64>>> = Vec::new();
    let mut standard_dev_of_n: f64 = 0.0;

    for i in 0..number_of_players {
        //generate player card and player card's win conditions:
        let (mut player, player_wins_true) = card_generate(card_size, row_range, free_space);
        player_list.push(player.clone());
        for j in 0..number_of_games {
            let mut player_wins = player_wins_true.clone(); //clones because otherwise the player_wins sheet does not refresh and auto wins
            let mut counter: i64 = 0;
            let mut forbidden_choice: Vec<i64> = Vec::new(); //Stores the values we have already pulled from the jar
            'round: while counter <= number_range {
                let mut choix: i64 = rand::random_range(1..=number_range as i64); //choix = French for "choice"
                'forbidden: while forbidden_choice.contains(&choix) {
                    choix = rand::random_range(1..=number_range as i64);
                    if counter == number_range {
                        //Stops after all numbers are pulled.
                        break 'forbidden;
                    }
                }
                //Converts called number on winning conditions sheet to a 0
                for j in 0..player_wins.len() {
                    for k in 0..player_wins[j].len() {
                        if player_wins[j][k] == choix {
                            player_wins[j][k] = 0;
                        }
                    }
                }
                //Converts called number on bingo sheet to a 0
                if verbose == true {
                    for j in 0..player.len() {
                        for k in 0..player.len() {
                            if player[j][k] == choix {
                                player[j][k] = 0;
                            }
                        }
                    }
                    println!("{:?}", player);
                }
                //A safety exit for the while loop
                if counter == number_range {
                    break 'round;
                }
                //exits the while loop when a win condition is met on the win sheet
                if player_wins.contains(&win_condition) {
                    break 'round;
                }
                forbidden_choice.push(choix); //Ensures same number cannot be pulled a second time.
                all_called_nums[i as usize].push(choix);
                counter += 1;
            }
            all_counters.push(counter);
            (horizontal_victory, vertical_victory, diagonal_victory) = determine_victory(
                horizontal_victory,
                vertical_victory,
                diagonal_victory,
                &player_wins,
                card_size,
            );
        }
    }
    if verbose == true {
        //printing controlled from toml file bool
        println!("{:?}", counter_average_vector);
        println!("{:?}", all_counters);
    }
    let total_wins = vertical_victory + horizontal_victory + diagonal_victory;
    //assesses contents of the 'statistics' array from the toml file
    if statistics.contains(&toml::Value::String(String::from("rounds"))) {
        counter_sample_mean = mean(all_counters.clone());
        standard_dev_of_n = std(all_counters.clone(), counter_sample_mean);
    } else {
        all_called_nums = Vec::new(); //returns null in the json output
        counter_sample_mean = 0.0 / 0.0;
        standard_dev_of_n = 0.0 / 0.0; 
    }
    if statistics.contains(&toml::Value::String(String::from("row%"))) {
        horizontal_victory_percentage = (horizontal_victory as f64 / total_wins as f64) * 100.0;
    } else {
        horizontal_victory_percentage = 0.0 / 0.0; //returns null
    }
    if statistics.contains(&toml::Value::String(String::from("col%"))) {
        vertical_victory_percentage = (vertical_victory as f64 / total_wins as f64) * 100.0;
    } else {
        vertical_victory_percentage = 0.0 / 0.0;
    }
    if statistics.contains(&toml::Value::String(String::from("diag%"))) {
        diagonal_victory_percentage = (diagonal_victory as f64 / total_wins as f64) * 100.0;
    } else {
        diagonal_victory_percentage = 0.0 / 0.0;
    }
    if verbose == true {
        finale(
            total_wins as i64,
            counter_sample_mean,
            standard_dev_of_n,
            diagonal_victory_percentage,
            vertical_victory_percentage,
            horizontal_victory_percentage,
        );
    }
    print_board_to_json(
        player_list,
        all_called_nums,
        counter_sample_mean,
        standard_dev_of_n,
        horizontal_victory_percentage,
        vertical_victory_percentage,
        diagonal_victory_percentage,
        output_file,
    );
}

fn mean(data: Vec<i64>) -> f64 {
    //Calculates the mean of the given vector
    let mut sum: f64 = 0.0;
    for i in 0..data.len() {
        sum += data[i] as f64;
    }
    let mean: f64 = sum / (data.len() as f64);
    return mean;
}

fn std(data: Vec<i64>, mean: f64) -> f64 {
    //calculates the standard deviation from the given vector and vector mean
    let mut sum: f64 = 0.0;
    for i in 0..data.len() {
        let squared = ((data[i] as f64) - mean).powf(2.0);
        sum += squared;
    }
    let sum_divide: f64 = sum / ((data.len() as f64) - 1.0);
    let standard_dev = sum_divide.powf(0.5);
    return standard_dev;
}

fn determine_victory(
    mut horizontal_victory: i64,
    mut vertical_victory: i64,
    mut diagonal_victory: i64,
    player_wins: &Vec<Vec<i64>>,
    card_size: i64,
) -> (i64, i64, i64) {
    //determines the type of victory, horizontal, vertical, or diagonal, of the given bingo card
    let win_condition: Vec<i64> = vec![0; card_size as usize];
    let mut win_index = 0;
    for i in 0..(2 * card_size + 2) {   //checks position of win_condition in the array of numbers that have all possible wins
        if player_wins[i as usize] == win_condition {
            win_index = i;
        }
    }
    if win_index < card_size {
        horizontal_victory += 1;
    } else if win_index >= card_size && win_index < 2 * card_size {
        vertical_victory += 1;
    } else {
        diagonal_victory += 1;
    }
    return (horizontal_victory, vertical_victory, diagonal_victory);
}

fn card_generate(
    card_size: i64,
    row_range: i64,
    free_space: bool,
) -> (Vec<Vec<i64>>, Vec<Vec<i64>>) {
    //Generates a card for the player
    let mut card: Vec<Vec<i64>> = vec![vec![]];
    let mut wins: Vec<Vec<i64>> = vec![vec![0; card_size as usize]; 2 * card_size as usize + 2];
    for _i in 0..(card_size - 1) {
        card.push(vec![]);
    }
    let mut forbidden_list: Vec<i64> = Vec::new();

    for i in 0..card_size {
        for j in 1..=card_size {
            //start at 1 bc if you don't, first column will be negative
            let mut x = rand::random_range((1 + (row_range * (j - 1)))..=(row_range * j));
            while forbidden_list.contains(&x) {
                x = rand::random_range((1 + (row_range * (j - 1)))..=(row_range * j));
                //println!("Stuck!");
            }
            forbidden_list.push(x.try_into().unwrap());
            card[i as usize].push(x);
            //NB: must make x mut so that the while loop x is stored at the same location
        }
    }
    if free_space && card_size % 2 == 1 {
        //Because the final card index will always be an even number when card_size%2=1,
        //the center location is always half of the length minus 1
        let center_location = card.len() / 2;
        card[center_location][center_location] = 0;
    }
    //Generate the win condition vector
    //wins = 2*card_size+2
    for i in 0..card_size {
        for j in 0..card_size {
            wins[i as usize][j as usize] = card[i as usize][j as usize]; // columns
            wins[card_size as usize + i as usize][j as usize] = card[j as usize][i as usize] // rows
        }
        wins[2 * card_size as usize][i as usize] = card[i as usize][i as usize]; // main diag
        wins[(2 * card_size + 1) as usize][i as usize] =
            card[i as usize][(card_size - i - 1) as usize]; // cross diag
    }
    return (card, wins);
}
#[derive(Serialize)]
struct JsonPrintSetup {
    all_player_cards: Vec<Vec<Vec<i64>>>,
    all_nums_pulled: Vec<Vec<i64>>,
    mean: f64,
    standard_deviation: f64,
    horizontal_win_percentage: f64,
    vertical_win_percentage: f64,
    diagonal_win_percentage: f64,
}

fn print_board_to_json(
    player_list: Vec<Vec<Vec<i64>>>,
    all_called_nums: Vec<Vec<i64>>,
    counter_sample_mean: f64,
    standard_dev_of_n: f64,
    horizontal_victory_percentage: f64,
    vertical_victory_percentage: f64,
    diagonal_victory_percentage: f64,
    output_file: &str,
) {
    let path = Path::new(output_file);
    let data_for_json = JsonPrintSetup {
        all_player_cards: player_list,
        all_nums_pulled: all_called_nums,
        mean: counter_sample_mean,
        standard_deviation: standard_dev_of_n,
        horizontal_win_percentage: horizontal_victory_percentage,
        vertical_win_percentage: vertical_victory_percentage,
        diagonal_win_percentage: diagonal_victory_percentage,
    };
    //converts data_for_json into a string to be put into the json file
    let json_data =
        serde_json::to_string(&data_for_json).expect("Failed to serialize data, sorry bro");

    let mut f = File::create(path).expect("Unable to create file");
    f.write_all(json_data.as_bytes())
        .expect("Failed to write JSON to file, whoopsies!");
}

fn finale(
    //prints the values after each game
    total_wins: i64,
    counter: f64,
    standard_dev_of_n: f64,
    diagonal_victory_percentage: f64,
    vertical_victory_percentage: f64,
    horizontal_victory_percentage: f64,
) {
    //prints all of the statistics in the terminal
    println!("Total Wins: {}", total_wins);
    println!(
        "Horizontal Victories: {:.2}%",
        horizontal_victory_percentage
    );
    println!("Vertical Victories: {:.2}%", vertical_victory_percentage);
    println!("Diagonal Victories: {:.2}%", diagonal_victory_percentage);
    println!("The standard deviation of N is {:.2}", standard_dev_of_n);
    println!("It took an average of {counter} draws to reach a victory.");
}
