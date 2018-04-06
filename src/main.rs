#![feature(lang_items)]
#![feature(const_fn)]
#![feature(alloc)]
#![feature(asm)]
#![feature(compiler_builtins_lib)]
#![no_std]
#![no_main]

#[macro_use]
extern crate stm32f7_discovery as stm32f7;

// initialization routines for .data and .bss

#[macro_use]
extern crate alloc;
extern crate compiler_builtins;
extern crate r0;
extern crate smoltcp;
extern crate arrayvec;

// hardware register structs with accessor methods
use stm32f7::{audio, board, embedded, lcd, sdram, system_clock, touch, i2c};

mod graphics;

#[no_mangle]
pub unsafe extern "C" fn reset() -> ! {
    extern "C" {
        static __DATA_LOAD: u32;
        static mut __DATA_END: u32;
        static mut __DATA_START: u32;

        static mut __BSS_START: u32;
        static mut __BSS_END: u32;
    }

    // initializes the .data section (copy the data segment initializers from flash to RAM)
    r0::init_data(&mut __DATA_START, &mut __DATA_END, &__DATA_LOAD);
    // zeroes the .bss section
    r0::zero_bss(&mut __BSS_START, &__BSS_END);

    stm32f7::heap::init();

    // enable floating point unit
    let scb = stm32f7::cortex_m::peripheral::scb_mut();
    scb.cpacr.modify(|v| v | 0b1111 << 20);
    asm!("DSB; ISB;"::::"volatile"); // pipeline flush

    main(board::hw());
}

// WORKAROUND: rust compiler will inline & reorder fp instructions into
#[inline(never)] //             reset() before the FPU is initialized
fn main(hw: board::Hardware) -> ! {
    use embedded::interfaces::gpio::{self, Gpio};

    let x = vec![1, 2, 3, 4, 5];
    assert_eq!(x.len(), 5);
    assert_eq!(x[3], 4);

    let board::Hardware {
        rcc,
        pwr,
        flash,
        fmc,
        ltdc,
        gpio_a,
        gpio_b,
        gpio_c,
        gpio_d,
        gpio_e,
        gpio_f,
        gpio_g,
        gpio_h,
        gpio_i,
        gpio_j,
        gpio_k,
        i2c_3,
        sai_2,
        syscfg,
        nvic,
        exti,
        ..
    } = hw;

    let mut gpio = Gpio::new(
        gpio_a,
        gpio_b,
        gpio_c,
        gpio_d,
        gpio_e,
        gpio_f,
        gpio_g,
        gpio_h,
        gpio_i,
        gpio_j,
        gpio_k,
    );

    system_clock::init(rcc, pwr, flash);

    // enable all gpio ports
    rcc.ahb1enr.update(|r| {
        r.set_gpioaen(true);
        r.set_gpioben(true);
        r.set_gpiocen(true);
        r.set_gpioden(true);
        r.set_gpioeen(true);
        r.set_gpiofen(true);
        r.set_gpiogen(true);
        r.set_gpiohen(true);
        r.set_gpioien(true);
        r.set_gpiojen(true);
        r.set_gpioken(true);
    });

    // configure led pin as output pin
    let led_pin = (gpio::Port::PortI, gpio::Pin::Pin1);
    let mut led = gpio.to_output(
        led_pin,
        gpio::OutputType::PushPull,
        gpio::OutputSpeed::Low,
        gpio::Resistor::NoPull,
    ).expect("led pin already in use");

    // turn led on
    led.set(true);

    let button_pin = (gpio::Port::PortI, gpio::Pin::Pin11);
    let _ = gpio.to_input(button_pin, gpio::Resistor::NoPull)
        .expect("button pin already in use");

    // init sdram (needed for display buffer)
    sdram::init(rcc, fmc, &mut gpio);

    // lcd controller
    let mut lcd = lcd::init(ltdc, rcc, &mut gpio);
    let mut layer_1 = lcd.layer_1().unwrap();
    let mut layer_2 = lcd.layer_2().unwrap();

    layer_1.clear();
    layer_2.clear();
    //lcd::init_stdout(layer_2);

    // i2c
    i2c::init_pins_and_clocks(rcc, &mut gpio);
    let mut i2c_3 = i2c::init(i2c_3);
    i2c_3.test_1();
    i2c_3.test_2();

    // sai and stereo microphone
    audio::init_sai_2_pins(&mut gpio);
    audio::init_sai_2(sai_2, rcc);
    assert!(audio::init_wm8994(&mut i2c_3).is_ok());

    touch::check_family_id(&mut i2c_3).unwrap();

    let mut audio_writer = layer_1.audio_writer();
    let mut text_writer = layer_2.text_writer();
    let mut last_led_toggle = system_clock::ticks();

    use stm32f7::board::embedded::components::gpio::stm32f7::Pin;
    use stm32f7::board::embedded::interfaces::gpio::Port;
    use stm32f7::exti::{EdgeDetection, Exti, ExtiLine};

    let mut exti = Exti::new(exti);
    let mut exti_handle = exti.register(
        ExtiLine::Gpio(Port::PortI, Pin::Pin11),
        EdgeDetection::FallingEdge,
        syscfg,
    ).unwrap();

    use stm32f7::interrupts::interrupt_request::InterruptRequest;
    use stm32f7::interrupts::{scope, Priority};

    scope(
        nvic,
        |_| {},
        |interrupt_table| {
            let _ =
                interrupt_table.register(InterruptRequest::Exti10to15, Priority::P1, move || {
                    exti_handle.clear_pending_state();
                    // choose a new background color
                    let new_color =
                        ((system_clock::ticks() as u32).wrapping_mul(19801)) % 0x1000000;
                    lcd.set_background_color(lcd::Color::from_hex(new_color));
                });

            /* let mut last_x = 0;
            let mut last_y = 0; */
            let color = stm32f7::lcd::Color::rgb(255, 255, 255);
            let mut duration_of_touch = 0;
            let mut cursor_model = graphics::model::CursorModel{first_contact: None, second_contact: None};
            let mut model = graphics::model::Model{p: graphics::point::Point{x:100, y:50}, r:20, cursor: cursor_model};
            loop {
                let ticks = system_clock::ticks();

                // every 0.5 seconds
                if ticks - last_led_toggle >= 500 {
                    // toggle the led
                    let led_current = led.get();
                    led.set(!led_current);
                    last_led_toggle = ticks;
                }

                /* let number_of_touches = touch::touches(&mut i2c_3).unwrap().len();
                    if number_of_touches as i32 == 1{
                        duration_of_touch += 1;
                        println!("duration of touch = {}", duration_of_touch);
                    } else if number_of_touches as i32 == 0 {
                        duration_of_touch = 0;
                    }  */

                //println!("test");
                /* text_writer.print_str_at(100, 100, "testString");
                text_writer.print_str_at(100, 150, "testString2\ntest"); */

                // poll for new touch data
                /* for touch in &touch::touches(&mut i2c_3).unwrap() {
                    let lcd = audio_writer.layer();
                    /* let p0 = graphics::point::Point{
                        x: last_x,
                        y: last_y,
                    };
                    let p1 = graphics::point::Point{
                        x: touch.x as usize,
                        y: touch.y as usize,
                    };
                    graphics::line::draw_line(lcd, &p0, &p1, color);
                    last_x = touch.x as usize;
                    last_y = touch.y as usize; */
                    // audio_writer.layer().print_point_at(touch.x as usize, touch.y as usize);
                } */

                model = graphics::update::update(model, &touch::touches(&mut i2c_3).unwrap());
                let lcd = audio_writer.layer();
                 graphics::view::view(&model, lcd);
            }
            
        },
    )
}
